//! 私有多文件 Maven Javadoc 原生探针；只重放静态简单项目的原 POM。

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use codeguard_adapters::{
    MavenJavadocParseState, javadoc_pom_direct_replay_eligible, parse_detailed_maven_javadoc_output,
};
use codeguard_runtime::{
    ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::tool_identity::hash_bundle_tree;

const SETTINGS: &str = include_str!("../resources/isolated_maven_settings.xml");

/// 多文件探针输入；所有路径相对同一个构建根。
pub(crate) struct Request<'a> {
    pub build_root: &'a Path,
    pub sources: &'a BTreeSet<String>,
    pub expected_pom_sha256: &'a str,
    pub maven_tool: Option<&'a Path>,
    pub java_home: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 在新私有项目副本中调用 Maven Javadoc 原生 goal；返回未授权的诊断观察。
pub(crate) fn observe(request: &Request<'_>) -> Value {
    let mut report = incomplete_observation("prerequisites_missing", request.sources.len());
    let (Some(maven), Some(java_home), Some(repo), Some(repo_digest)) = (
        request.maven_tool,
        request.java_home,
        request.maven_repo,
        request.repo_sha256,
    ) else {
        return report;
    };
    if request.cancelled.load(Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        return with_reason(report, "request_cancelled");
    }
    if Instant::now() >= request.deadline {
        return with_reason(report, "request_deadline_exceeded");
    }
    if request.sources.is_empty() || request.sources.iter().any(|path| !is_main_java(path)) {
        return with_reason(report, "main_java_source_scope_invalid");
    }
    let mut relative: Vec<_> = request.sources.iter().map(PathBuf::from).collect();
    relative.push(PathBuf::from("pom.xml"));
    let Ok(snapshot) = SourceSnapshot::capture(
        request.build_root,
        relative,
        2_001,
        16 * 1024 * 1024,
        128 * 1024 * 1024,
    ) else {
        return with_reason(report, "source_snapshot_unavailable");
    };
    let Some(pom) = snapshot.files().get(Path::new("pom.xml")) else {
        return with_reason(report, "project_pom_unavailable");
    };
    if digest(pom) != request.expected_pom_sha256 {
        return with_reason(report, "project_pom_changed_before_scan");
    }
    if !javadoc_pom_direct_replay_eligible(pom) {
        return with_reason(report, "project_pom_replay_ineligible");
    }
    report["native_plan_sha256"] = json!(digest(pom));
    report["pom_mode"] = json!("direct_pom_replay");
    let (Ok(maven), Ok(java_home), Ok(repo)) = (
        maven.canonicalize(),
        java_home.canonicalize(),
        repo.canonicalize(),
    ) else {
        return with_reason(report, "native_prerequisite_unavailable");
    };
    let java = java_home.join("bin/java");
    let (Ok(maven_bytes), Ok(java_bytes), Ok(release_bytes)) = (
        read_bounded_regular_file(&maven, 128 * 1024 * 1024),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
        read_bounded_regular_file(&java_home.join("release"), 64 * 1024),
    ) else {
        return with_reason(report, "native_tool_identity_unavailable");
    };
    if !std::str::from_utf8(&release_bytes)
        .ok()
        .is_some_and(|text| {
            text.lines()
                .any(|line| line.starts_with("JAVA_VERSION=\"21."))
        })
    {
        return with_reason(report, "jdk_version_unverified");
    }
    if repo_digest.len() != 64
        || !repo_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_digest)
    {
        return with_reason(report, "dependency_closure_mismatch");
    }
    report["maven_tool_sha256"] = json!(digest(&maven_bytes));
    report["java_runtime_sha256"] = json!(digest(&java_bytes));
    report["dependency_closure_sha256"] = json!(repo_digest);
    let Ok(scratch) = private_scratch() else {
        return with_reason(report, "private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    if snapshot.materialize_new(&project).is_err()
        || fs::write(project.join("settings.xml"), SETTINGS).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
    {
        return with_reason(report, "private_workspace_unavailable");
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
            "-o".into(),
            "-s".into(),
            project.join("settings.xml").into_os_string(),
            "-gs".into(),
            project.join("settings.xml").into_os_string(),
            format!("-Dmaven.repo.local={}", repo.display()).into(),
            "org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc".into(),
        ],
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let outcome = match run_process_recorded(&spec, request.cancelled, &evidence, "javadoc.log") {
        Ok(outcome) => outcome,
        Err(_) => return with_reason(report, "native_process_incomplete"),
    };
    if snapshot.verify_unchanged(&project).ok() != Some(true)
        || read_bounded_regular_file(&project.join("settings.xml"), 64 * 1024)
            .ok()
            .as_deref()
            != Some(SETTINGS.as_bytes())
    {
        return with_reason(report, "native_inputs_changed_during_scan");
    }
    if read_bounded_regular_file(&maven, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(maven_bytes.as_slice())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || read_bounded_regular_file(&java_home.join("release"), 64 * 1024)
            .ok()
            .as_deref()
            != Some(release_bytes.as_slice())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_digest)
    {
        return with_reason(report, "native_identity_changed_during_scan");
    }
    let Some(exit) = (match outcome.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    }) else {
        return with_reason(report, "native_execution_incomplete");
    };
    if !outcome.stderr.is_empty() {
        return with_reason(report, "native_stderr_unrecognized");
    }
    let source_map: BTreeMap<String, Vec<u8>> = snapshot
        .files()
        .iter()
        .filter(|(path, _)| path.as_path() != Path::new("pom.xml"))
        .map(|(path, bytes)| {
            (
                project.join(path).to_string_lossy().into_owned(),
                bytes.clone(),
            )
        })
        .collect();
    let parsed = parse_detailed_maven_javadoc_output(&outcome.stdout, exit, "3.12.0", &source_map);
    if parsed.state == MavenJavadocParseState::Incomplete {
        return with_reason(
            report,
            parsed.reason.unwrap_or("native_output_unrecognized"),
        );
    }
    if read_bounded_regular_file(
        &project.join("target/reports/apidocs/index.html"),
        16 * 1024 * 1024,
    )
    .is_err()
    {
        return with_reason(report, "native_output_missing");
    }
    report["findings"] = json!(
        parsed
            .diagnostics
            .iter()
            .filter_map(|item| {
                let copied = Path::new(&item.path);
                let relative = copied.strip_prefix(&project).ok()?;
                let source = snapshot.files().get(relative)?;
                let line = source
                    .split(|byte| *byte == b'\n')
                    .nth((item.line - 1) as usize)?;
                ((item.column as usize) <= line.len() + 1).then(|| {
                    json!({
                        "path":relative.to_string_lossy(), "line":item.line,
                        "column":item.column, "rule_id":item.rule_id
                    })
                })
            })
            .collect::<Vec<_>>()
    );
    if report["findings"].as_array().map(Vec::len) != Some(parsed.diagnostics.len()) {
        return with_reason(report, "native_location_invalid");
    }
    report["observed_source_count"] = json!(request.sources.len());
    report["native_status"] = json!(match parsed.state {
        MavenJavadocParseState::ValidDiagnostics => "findings_observed_untrusted",
        MavenJavadocParseState::CleanLogUnverified => "clean_log_unverified",
        MavenJavadocParseState::Incomplete => unreachable!(),
    });
    with_reason(report, "direct_pom_scope_and_effective_model_unverified")
}

/// 为检查前配置变化生成同形状的未完成观察，避免宿主得到无法解析的局部对象。
pub(crate) fn incomplete_observation(reason: &'static str, source_count: usize) -> Value {
    json!({
        "schema_version":"0.2.0", "report_type":"maven_javadoc_multifile_probe",
        "checker_id":"java.maven.javadoc", "native_status":"incomplete",
        "reason":reason, "source_count":source_count,
        "observed_source_count":0, "findings":[], "native_plan_sha256":null,
        "pom_mode":"unverified",
        "maven_tool_sha256":null, "java_runtime_sha256":null, "dependency_closure_sha256":null,
        "project_checker_attribution":"unverified", "coverage_proven":false,
        "authority":"local_unverified", "delivery_decision":"not_evaluated"
    })
}

fn is_main_java(path: &str) -> bool {
    path.starts_with("src/main/java/") && path.ends_with(".java")
}

fn with_reason(mut report: Value, reason: &'static str) -> Value {
    report["reason"] = json!(reason);
    report
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn private_scratch() -> Result<Scratch, &'static str> {
    let temp = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "temp_root_unavailable")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let path = temp.join(format!(
        "codeguard-maven-javadoc-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).map_err(|_| "scratch_create_failed")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "scratch_permission_failed")?;
    Ok(Scratch(path))
}

#[cfg(test)]
mod tests {
    use super::{Request, observe, private_scratch};
    use sha2::Digest;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    #[test]
    fn parent_pom_does_not_launch_a_synthetic_project_probe() {
        let scratch = private_scratch().unwrap();
        let root = scratch.0.join("project");
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(
            root.join("src/main/java/Demo.java"),
            b"public class Demo {}\n",
        )
        .unwrap();
        fs::write(root.join("pom.xml"), b"<project><parent/><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>").unwrap();
        let sources = BTreeSet::from(["src/main/java/Demo.java".to_owned()]);
        let cancelled = AtomicBool::new(false);
        let request = Request {
            build_root: &root,
            sources: &sources,
            expected_pom_sha256: &format!(
                "{:x}",
                sha2::Sha256::digest(fs::read(root.join("pom.xml")).unwrap())
            ),
            maven_tool: Some(Path::new("/nonexistent/maven")),
            java_home: Some(Path::new("/nonexistent/jdk")),
            maven_repo: Some(Path::new("/nonexistent/repo")),
            repo_sha256: Some("0"),
            deadline: Instant::now() + Duration::from_secs(5),
            cancelled: &cancelled,
        };
        let report = observe(&request);
        assert_eq!(report["native_status"], "incomplete");
        assert_eq!(report["reason"], "project_pom_replay_ineligible");
        assert_eq!(report["findings"], serde_json::json!([]));
        assert_eq!(report["pom_mode"], "unverified");
        let changed = observe(&Request {
            expected_pom_sha256: &"f".repeat(64),
            ..request
        });
        assert_eq!(changed["reason"], "project_pom_changed_before_scan");
        assert_eq!(changed["findings"], serde_json::json!([]));
    }

    /// 生成代码（如 target/generated-sources）不在原 POM 主源码范围内；
    /// 探针必须拒绝启动而不是为其合成诊断。
    #[test]
    fn generated_sources_outside_main_java_never_launch_the_probe() {
        let scratch = private_scratch().unwrap();
        let root = scratch.0.join("project");
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(
            root.join("src/main/java/Demo.java"),
            b"public class Demo {}\n",
        )
        .unwrap();
        fs::write(root.join("pom.xml"), b"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>").unwrap();
        let cancelled = AtomicBool::new(false);
        let pom_digest = format!(
            "{:x}",
            sha2::Sha256::digest(fs::read(root.join("pom.xml")).unwrap())
        );
        for generated in [
            "target/generated-sources/Gen.java",
            "src/test/java/Gen.java",
            "Gen.java",
        ] {
            let sources =
                BTreeSet::from(["src/main/java/Demo.java".to_owned(), generated.to_owned()]);
            let request = Request {
                build_root: &root,
                sources: &sources,
                expected_pom_sha256: &pom_digest,
                maven_tool: Some(Path::new("/nonexistent/maven")),
                java_home: Some(Path::new("/nonexistent/jdk")),
                maven_repo: Some(Path::new("/nonexistent/repo")),
                repo_sha256: Some("0"),
                deadline: Instant::now() + Duration::from_secs(5),
                cancelled: &cancelled,
            };
            let report = observe(&request);
            assert_eq!(
                report["native_status"], "incomplete",
                "{generated}: {report}"
            );
            assert_eq!(report["reason"], "main_java_source_scope_invalid");
            assert_eq!(report["findings"], serde_json::json!([]));
            assert_eq!(report["observed_source_count"], 0);
            // native_plan_sha256 未设置说明从未进入原生执行路径。
            assert!(report["native_plan_sha256"].is_null(), "{generated}");
        }
    }

    /// 快照条目数超过 2001 上限时保持未完成，不裁剪输入。
    #[test]
    fn more_than_two_thousand_entries_exceed_the_snapshot_budget() {
        let scratch = private_scratch().unwrap();
        let root = scratch.0.join("project");
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        let mut sources = BTreeSet::new();
        for index in 0..2002 {
            let path = root.join(format!("src/main/java/S{index:04}.java"));
            fs::write(&path, b"public class S {}\n").unwrap();
            sources.insert(format!("src/main/java/S{index:04}.java"));
        }
        fs::write(root.join("pom.xml"), b"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>demo</artifactId><version>1</version></project>").unwrap();
        let cancelled = AtomicBool::new(false);
        let pom_digest = format!(
            "{:x}",
            sha2::Sha256::digest(fs::read(root.join("pom.xml")).unwrap())
        );
        let report = observe(&Request {
            build_root: &root,
            sources: &sources,
            expected_pom_sha256: &pom_digest,
            maven_tool: Some(Path::new("/nonexistent/maven")),
            java_home: Some(Path::new("/nonexistent/jdk")),
            maven_repo: Some(Path::new("/nonexistent/repo")),
            repo_sha256: Some("0"),
            deadline: Instant::now() + Duration::from_secs(30),
            cancelled: &cancelled,
        });
        assert_eq!(report["native_status"], "incomplete", "{report}");
        assert_eq!(report["reason"], "source_snapshot_unavailable");
        assert_eq!(report["findings"], serde_json::json!([]));
    }
}
