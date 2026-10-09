//! 原生 OWASP Maven 局部扫描；保留漏洞观察，数据库时效未知时不签发清洁结论。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use codeguard_adapters::{owasp_maven_pom_plan, parse_owasp_dependency_check_json};
use codeguard_runtime::{
    ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::tool_identity::hash_bundle_tree;

/// 真实 dependency-check 插件的 goal 声明 `requiresOnline`，Maven `-o` 会让每次执行
/// 在 goal 启动前直接失败。因此本探针以在线模式调用 Maven，但隔离 settings 的镜像
/// 指向不可路由地址：离线仓库闭包完整时零联网，缺件时确定性解析失败，绝不静默下载。
const SETTINGS: &str = "<settings xmlns=\"http://maven.apache.org/SETTINGS/1.2.0\"><mirrors><mirror><id>nexus-aliyun</id><url>http://127.0.0.1:9/</url><mirrorOf>*</mirrorOf></mirror></mirrors></settings>";

/// 原生检查请求；离线数据库和 Maven 闭包须有调用方提供的当前字节摘要。
pub(crate) struct Request<'a> {
    pub build_root: &'a Path,
    pub expected_pom_sha256: &'a str,
    pub maven_tool: Option<&'a Path>,
    pub java_home: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub data_dir: Option<&'a Path>,
    pub data_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 执行已声明的原生 OWASP goal，并只返回脱敏的局部观察。
pub(crate) fn observe(request: &Request<'_>) -> Value {
    let mut report = incomplete("prerequisites_missing");
    if request.cancelled.load(Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        return reason(report, "request_cancelled");
    }
    if Instant::now() >= request.deadline {
        return reason(report, "request_deadline_exceeded");
    }
    let Ok(snapshot) = SourceSnapshot::capture(
        request.build_root,
        [PathBuf::from("pom.xml")],
        1,
        4 * 1024 * 1024,
        4 * 1024 * 1024,
    ) else {
        return reason(report, "project_pom_unavailable");
    };
    let pom = snapshot
        .files()
        .get(Path::new("pom.xml"))
        .expect("POM 已捕获");
    if digest(pom) != request.expected_pom_sha256 {
        return reason(report, "project_pom_changed_before_scan");
    }
    let Some(plan) = owasp_maven_pom_plan(pom) else {
        return reason(report, "project_pom_replay_ineligible");
    };
    report["native_plan_sha256"] = json!(digest(pom));
    report["plugin_version"] = json!(plan.plugin_version);
    let (Some(maven), Some(java_home), Some(repo), Some(repo_sha), Some(data), Some(data_sha)) = (
        request.maven_tool,
        request.java_home,
        request.maven_repo,
        request.repo_sha256,
        request.data_dir,
        request.data_sha256,
    ) else {
        return report;
    };
    let (Ok(maven), Ok(java_home), Ok(repo), Ok(data)) = (
        maven.canonicalize(),
        java_home.canonicalize(),
        repo.canonicalize(),
        data.canonicalize(),
    ) else {
        return reason(report, "native_prerequisite_unavailable");
    };
    let java = java_home.join("bin/java");
    let (Ok(maven_bytes), Ok(java_bytes), Ok(release_bytes)) = (
        read_bounded_regular_file(&maven, 128 * 1024 * 1024),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
        read_bounded_regular_file(&java_home.join("release"), 64 * 1024),
    ) else {
        return reason(report, "native_tool_identity_unavailable");
    };
    if release_bytes.is_empty() {
        return reason(report, "jdk_version_unverified");
    }
    if !valid_digest(repo_sha)
        || !valid_digest(data_sha)
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_sha)
        || hash_bundle_tree(&data).ok().as_deref() != Some(data_sha)
    {
        return reason(report, "native_identity_mismatch");
    }
    report["maven_tool_sha256"] = json!(digest(&maven_bytes));
    report["java_runtime_sha256"] = json!(digest(&java_bytes));
    report["dependency_closure_sha256"] = json!(repo_sha);
    report["database_tree_sha256"] = json!(data_sha);
    let Ok(scratch) = private_scratch() else {
        return reason(report, "private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    let database = scratch.0.join("database");
    if snapshot.materialize_new(&project).is_err()
        || fs::write(project.join("settings.xml"), SETTINGS).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
        || copy_bounded_tree(&data, &database, request.deadline).is_err()
        || hash_bundle_tree(&database).ok().as_deref() != Some(data_sha)
        || hash_bundle_tree(&data).ok().as_deref() != Some(data_sha)
    {
        return reason(report, "private_database_snapshot_unavailable");
    }
    if Instant::now() >= request.deadline {
        return reason(report, "request_deadline_exceeded");
    }
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_owned(),
    );
    env.insert(OsString::from("MAVEN_SKIP_RC"), OsString::from("1"));
    env.insert(
        OsString::from("PATH"),
        OsString::from(format!("{}:/usr/bin:/bin", java_home.join("bin").display())),
    );
    let spec = ProcessSpec {
        executable: maven.clone(),
        args: vec![
            "-B".into(),
            "-ntp".into(),
            "-s".into(),
            project.join("settings.xml").into_os_string(),
            "-gs".into(),
            project.join("settings.xml").into_os_string(),
            format!("-Dmaven.repo.local={}", repo.display()).into(),
            format!(
                "org.owasp:dependency-check-maven:{}:check",
                plan.plugin_version
            )
            .into(),
            "-Dformat=JSON".into(),
            format!("-Dodc.outputDirectory={}", evidence.display()).into(),
            format!("-DdataDirectory={}", database.display()).into(),
            "-DautoUpdate=false".into(),
            "-DcentralAnalyzerEnabled=false".into(),
            "-DossIndexAnalyzerEnabled=false".into(),
            "-DhostedSuppressionsEnabled=false".into(),
        ],
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let outcome = match run_process_recorded(&spec, request.cancelled, &evidence, "owasp.log") {
        Ok(value) => value,
        Err(_) => return reason(report, "native_process_incomplete"),
    };
    if snapshot.verify_unchanged(&project).ok() != Some(true)
        || read_bounded_regular_file(&project.join("settings.xml"), 64 * 1024)
            .ok()
            .as_deref()
            != Some(SETTINGS.as_bytes())
    {
        return reason(report, "native_inputs_changed_during_scan");
    }
    if read_bounded_regular_file(&maven, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(maven_bytes.as_slice())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_sha)
        || hash_bundle_tree(&data).ok().as_deref() != Some(data_sha)
        || hash_bundle_tree(&database).ok().as_deref() != Some(data_sha)
    {
        return reason(report, "native_identity_changed_during_scan");
    }
    if !matches!(outcome.termination, Termination::Exited(0)) || !outcome.stderr.is_empty() {
        return reason(report, "native_execution_incomplete");
    }
    if validate_log(&outcome.stdout, &plan.plugin_version, &plan.artifact_id).is_err() {
        return reason(report, "native_goal_unverified");
    }
    let Ok(bytes) = read_bounded_regular_file(
        &evidence.join("dependency-check-report.json"),
        8 * 1024 * 1024,
    ) else {
        return reason(report, "native_report_missing");
    };
    let Ok(parsed) = parse_owasp_dependency_check_json(&bytes) else {
        return reason(report, "native_report_invalid");
    };
    if parsed.engine_version != plan.plugin_version {
        return reason(report, "native_report_tool_mismatch");
    }
    if parsed.project_name != plan.artifact_id {
        return reason(report, "native_report_project_mismatch");
    }
    if Instant::now() >= request.deadline {
        return reason(report, "request_deadline_exceeded");
    }
    report["native_report_sha256"] = json!(digest(&bytes));
    report["dependency_count"] = json!(parsed.dependency_count);
    report["database_sources"] = json!(
        parsed
            .data_sources
            .iter()
            .map(|(name, timestamp)| json!({"name":safe_label(name),"timestamp":safe_timestamp(timestamp)}))
            .collect::<Vec<_>>()
    );
    report["advisories"] = json!(
        parsed
            .advisories
            .iter()
            .map(|item| json!({
                "source":safe_label(&item.source),"advisory_id":safe_label(&item.advisory_id),"score":item.score,
                "package_ids":item.package_ids.iter().map(|value| safe_package_id(value)).collect::<Vec<_>>(),"dependency_sha256":item.dependency_sha256,
                "suppressed_by_native_tool":item.suppressed_by_native_tool
            }))
            .collect::<Vec<_>>()
    );
    report["native_status"] = json!(if parsed.advisories.is_empty() {
        "empty_report_unverified"
    } else {
        "findings_observed_untrusted"
    });
    report["reason"] = json!("database_freshness_and_project_coverage_unverified");
    report
}

/// 所有失败路径保留同形状空观察，绝不把未执行呈现成零漏洞。
pub(crate) fn incomplete(why: &'static str) -> Value {
    json!({
        "schema_version":"0.1.0","report_type":"owasp_maven_probe",
        "checker_id":"java.maven.dependency_check","category":"cve",
        "native_status":"incomplete","reason":why,"plugin_version":null,
        "native_plan_sha256":null,"maven_tool_sha256":null,"java_runtime_sha256":null,
        "dependency_closure_sha256":null,"database_tree_sha256":null,
        "native_report_sha256":null,"database_freshness":"unverified",
        "dependency_count":0,"database_sources":[],"advisories":[],
        "coverage_proven":false,"authority":"local_unverified",
        "delivery_decision":"not_evaluated"
    })
}

fn reason(mut report: Value, why: &'static str) -> Value {
    report["reason"] = json!(why);
    report
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn redacted(value: &str) -> String {
    format!("redacted-sha256:{}", digest(value.as_bytes()))
}

fn safe_label(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b' ')
        })
    {
        value.to_owned()
    } else {
        redacted(value)
    }
}

fn safe_timestamp(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= 40
        && value.bytes().all(|byte| {
            byte.is_ascii_digit()
                || matches!(byte, b'T' | b'Z' | b't' | b'z' | b'-' | b':' | b'.' | b'+')
        })
    {
        value.to_owned()
    } else {
        redacted(value)
    }
}

fn safe_package_id(value: &str) -> String {
    fn component(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 200
            && value != "."
            && value != ".."
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+')
            })
    }
    let Some(rest) = value.strip_prefix("pkg:maven/") else {
        return redacted(value);
    };
    let (coordinate, qualifiers) = rest.split_once('?').unwrap_or((rest, ""));
    let Some((namespace, artifact_version)) = coordinate.split_once('/') else {
        return redacted(value);
    };
    let Some((artifact, version)) = artifact_version.split_once('@') else {
        return redacted(value);
    };
    let safe_qualifiers = qualifiers.is_empty()
        || qualifiers.split('&').all(|entry| {
            let Some((key, part)) = entry.split_once('=') else {
                return false;
            };
            matches!(key, "type" | "classifier") && component(part)
        });
    if value.len() <= 256
        && component(namespace)
        && component(artifact)
        && component(version)
        && safe_qualifiers
    {
        value.to_owned()
    } else {
        redacted(value)
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_log(bytes: &[u8], version: &str, artifact: &str) -> Result<(), ()> {
    let log = std::str::from_utf8(bytes).map_err(|_| ())?;
    if log.is_empty()
        || log
            .bytes()
            .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\n' | b'\r' | b'\t'))
        || log
            .lines()
            .any(|line| line.starts_with("[ERROR]") || line.starts_with("[WARNING]"))
    {
        return Err(());
    }
    let marker = format!("[INFO] --- dependency-check:{version}:check ");
    if log
        .lines()
        .filter(|line| line.starts_with(&marker) && line.contains(&format!(" @ {artifact} ---")))
        .count()
        != 1
        || log
            .lines()
            .filter(|line| *line == "[INFO] BUILD SUCCESS")
            .count()
            != 1
    {
        return Err(());
    }
    Ok(())
}

fn private_scratch() -> Result<Scratch, &'static str> {
    let root = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "temp_root_unavailable")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let path = root.join(format!("codeguard-owasp-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).map_err(|_| "scratch_create_failed")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "scratch_permission_failed")?;
    Ok(Scratch(path))
}

fn copy_bounded_tree(source: &Path, target: &Path, deadline: Instant) -> Result<(), ()> {
    fs::create_dir(target).map_err(|_| ())?;
    fs::set_permissions(target, fs::Permissions::from_mode(0o700)).map_err(|_| ())?;
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];
    let (mut entries, mut bytes_seen) = (0usize, 0u64);
    while let Some((from, to)) = pending.pop() {
        let children = fs::read_dir(from).map_err(|_| ())?;
        for child in children {
            if Instant::now() >= deadline || entries >= 10_000 {
                return Err(());
            }
            entries += 1;
            let child = child.map_err(|_| ())?;
            let source_child = child.path();
            let target_child = to.join(child.file_name());
            let kind = fs::symlink_metadata(&source_child)
                .map_err(|_| ())?
                .file_type();
            if kind.is_dir() {
                fs::create_dir(&target_child).map_err(|_| ())?;
                fs::set_permissions(&target_child, fs::Permissions::from_mode(0o700))
                    .map_err(|_| ())?;
                pending.push((source_child, target_child));
            } else if kind.is_file() {
                let content =
                    read_bounded_regular_file(&source_child, 128 * 1024 * 1024).map_err(|_| ())?;
                bytes_seen = bytes_seen.saturating_add(content.len() as u64);
                if bytes_seen > 512 * 1024 * 1024 {
                    return Err(());
                }
                fs::write(&target_child, content).map_err(|_| ())?;
                fs::set_permissions(&target_child, fs::Permissions::from_mode(0o600))
                    .map_err(|_| ())?;
            } else {
                return Err(());
            }
        }
    }
    if entries == 0 {
        return Err(());
    }
    Ok(())
}
