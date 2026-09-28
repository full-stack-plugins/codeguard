//! Java 单文件的原生 Maven/P3C 诊断入口；只保留局部事实，不签发项目质量结论。

use crate::tool_identity::hash_bundle_tree;
use codeguard_adapters::{PmdParseState, p3c_rule_in_selected_rulesets, parse_pmd_xml};
use codeguard_runtime::{
    ProcessSpec, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DECLARED_P3C_RULESETS: [&str; 10] = [
    "rulesets/java/ali-comment.xml",
    "rulesets/java/ali-concurrent.xml",
    "rulesets/java/ali-constant.xml",
    "rulesets/java/ali-exception.xml",
    "rulesets/java/ali-flowcontrol.xml",
    "rulesets/java/ali-naming.xml",
    "rulesets/java/ali-oop.xml",
    "rulesets/java/ali-orm.xml",
    "rulesets/java/ali-other.xml",
    "rulesets/java/ali-set.xml",
];

pub(crate) struct Args {
    pub(crate) source: PathBuf,
    pub(crate) maven_tool: Option<PathBuf>,
    pub(crate) java_home: Option<PathBuf>,
    pub(crate) maven_repo: Option<PathBuf>,
    pub(crate) repo_sha256: Option<String>,
    pub(crate) selected_rulesets: Option<Vec<&'static str>>,
    pub(crate) json: bool,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 调用显式指定的原生 Maven/P3C 组合检查一份 Java 源码，始终返回未完成门禁。
pub fn run(args: &[String]) -> ExitCode {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let report = observe(
        &args,
        Instant::now() + Duration::from_secs(120),
        &AtomicBool::new(false),
    );
    if args.json {
        println!("{report}");
    } else {
        println!(
            "Java P3C 单文件原生诊断：{}",
            report["local_status"].as_str().unwrap_or("incomplete")
        );
        println!("待核实：{}", report["reason"].as_str().unwrap_or("unknown"));
        for finding in report["findings"].as_array().into_iter().flatten() {
            println!(
                "{}:{} {}",
                finding["path"].as_str().unwrap_or("<unknown>"),
                finding["line"].as_u64().unwrap_or(0),
                finding["rule_id"].as_str().unwrap_or("<unknown>")
            );
        }
        println!("可信工具/规则/策略和完整源码范围尚未核验，不能签发质量通过。");
    }
    ExitCode::from(3)
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args, String> {
    let Some(source) = args.first() else {
        return Err("lint java 缺少 Java 源文件路径".into());
    };
    if source.starts_with('-') {
        return Err("lint java 缺少 Java 源文件路径".into());
    }
    let mut parsed = Args {
        source: PathBuf::from(source),
        maven_tool: None,
        java_home: None,
        maven_repo: None,
        repo_sha256: None,
        selected_rulesets: None,
        json: false,
    };
    let mut index = 1;
    while index < args.len() {
        let option = args[index].as_str();
        let (key, value) = if let Some((key, value)) = option.split_once('=') {
            (key, value.to_owned())
        } else {
            index += 1;
            (
                option,
                args.get(index).ok_or(format!("{option} 缺少值"))?.clone(),
            )
        };
        match key {
            "--maven-tool" if parsed.maven_tool.is_none() => {
                parsed.maven_tool = Some(PathBuf::from(value))
            }
            "--java-home" if parsed.java_home.is_none() => {
                parsed.java_home = Some(PathBuf::from(value))
            }
            "--maven-repo" if parsed.maven_repo.is_none() => {
                parsed.maven_repo = Some(PathBuf::from(value))
            }
            "--repo-sha256" if parsed.repo_sha256.is_none() => parsed.repo_sha256 = Some(value),
            "--format" if matches!(value.as_str(), "human" | "json") => {
                parsed.json = value == "json"
            }
            _ => return Err(format!("lint java 参数无效或重复：{key}")),
        }
        index += 1;
    }
    Ok(parsed)
}

/// 在调用方的统一截止时间及取消标志下观察一份 Java 源码。
pub(crate) fn observe(args: &Args, deadline: Instant, cancelled: &AtomicBool) -> Value {
    let selected = args
        .selected_rulesets
        .as_deref()
        .unwrap_or(&DECLARED_P3C_RULESETS);
    let native_pom = render_pom(selected);
    let source = args.source.canonicalize().ok();
    let source_path = source
        .as_deref()
        .unwrap_or(&args.source)
        .to_string_lossy()
        .into_owned();
    let mut report = json!({
        "schema_version":"0.2.0", "report_type":"java_p3c_local_feedback",
        "operation":"lint", "language":"java", "request_id":format!("java-p3c-{}", std::process::id()),
        "command_status":"incomplete", "exit_code":3, "local_status":"incomplete",
        "reason":"prerequisites_missing", "path":source_path, "source_sha256":null,
        "maven_tool_sha256":null, "java_runtime_sha256":null, "dependency_closure_sha256":null,
        "checker_identity":"unverified", "rulepack_approval":"unverified",
        "declared_rulesets":selected,
        "native_plan_sha256":digest(native_pom.as_bytes()),
        "coverage_proven":false, "findings":[], "delivery_decision":"not_evaluated",
        "warnings":["local_native_observation_only"],
        "next_actions":["bind_approved_tool_rulepack_and_quality_policy","verify_complete_source_coverage"]
    });
    let (Some(source), Some(tool), Some(java_home), Some(repo), Some(expected_repo)) = (
        source,
        args.maven_tool.as_ref(),
        args.java_home.as_ref(),
        args.maven_repo.as_ref(),
        args.repo_sha256.as_deref(),
    ) else {
        return report;
    };
    if cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        return with_reason(report, "request_cancelled");
    }
    if Instant::now() >= deadline {
        return with_reason(report, "request_deadline_exceeded");
    }
    if source.extension().and_then(|extension| extension.to_str()) != Some("java") {
        return with_reason(report, "source_not_java_file");
    }
    let Ok(source_bytes) = read_bounded_regular_file(&source, 16 * 1024 * 1024) else {
        return with_reason(report, "source_unavailable");
    };
    let source_digest = digest(&source_bytes);
    report["source_sha256"] = json!(source_digest);
    let (Ok(tool), Ok(java_home), Ok(repo)) = (
        tool.canonicalize(),
        java_home.canonicalize(),
        repo.canonicalize(),
    ) else {
        return with_reason(report, "native_prerequisite_unavailable");
    };
    let Ok(tool_bytes) = read_bounded_regular_file(&tool, 128 * 1024 * 1024) else {
        return with_reason(report, "maven_tool_unavailable");
    };
    let tool_digest = digest(&tool_bytes);
    report["maven_tool_sha256"] = json!(tool_digest);
    let java = java_home.join("bin/java");
    let Ok(java_bytes) = read_bounded_regular_file(&java, 128 * 1024 * 1024) else {
        return with_reason(report, "java_runtime_unavailable");
    };
    let java_digest = digest(&java_bytes);
    report["java_runtime_sha256"] = json!(java_digest);
    if expected_repo.len() != 64
        || !expected_repo
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return with_reason(report, "dependency_closure_digest_invalid");
    }
    if hash_bundle_tree(&repo).ok().as_deref() != Some(expected_repo) {
        return with_reason(report, "dependency_closure_mismatch");
    }
    report["dependency_closure_sha256"] = json!(expected_repo);
    let Ok(scratch) = private_scratch() else {
        return with_reason(report, "private_workspace_unavailable");
    };
    let copied_dir = scratch.0.join("src/main/java");
    if fs::create_dir_all(&copied_dir).is_err() {
        return with_reason(report, "private_workspace_unavailable");
    }
    let Some(name) = source.file_name() else {
        return with_reason(report, "source_unavailable");
    };
    let copied_source = copied_dir.join(name);
    if fs::write(&copied_source, &source_bytes).is_err()
        || fs::write(scratch.0.join("pom.xml"), native_pom.as_bytes()).is_err()
        || fs::write(
            scratch.0.join("settings.xml"),
            include_bytes!("../resources/isolated_maven_settings.xml"),
        )
        .is_err()
    {
        return with_reason(report, "private_workspace_unavailable");
    }
    let evidence_dir = scratch.0.join("evidence");
    if fs::create_dir(&evidence_dir).is_err()
        || fs::set_permissions(&evidence_dir, fs::Permissions::from_mode(0o700)).is_err()
    {
        return with_reason(report, "private_workspace_unavailable");
    }
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_os_string(),
    );
    env.insert(OsString::from("MAVEN_SKIP_RC"), OsString::from("1"));
    env.insert(
        OsString::from("PATH"),
        OsString::from(format!("{}:/usr/bin:/bin", java_home.join("bin").display())),
    );
    let spec = ProcessSpec {
        executable: tool.clone(),
        args: vec![
            "-B".into(),
            "-ntp".into(),
            "-q".into(),
            "-o".into(),
            "-s".into(),
            scratch.0.join("settings.xml").into_os_string(),
            "-gs".into(),
            scratch.0.join("settings.xml").into_os_string(),
            format!("-Dmaven.repo.local={}", repo.display()).into(),
            "org.apache.maven.plugins:maven-pmd-plugin:3.11.0:pmd".into(),
        ],
        cwd: scratch.0.clone(),
        env,
        stdin: None,
        deadline,
        output_limit_bytes: 1024 * 1024,
    };
    let outcome = match run_process_recorded(&spec, cancelled, &evidence_dir, "maven-p3c.log") {
        Ok(outcome) => outcome,
        Err(_) => return with_reason(report, "native_process_incomplete"),
    };
    if read_bounded_regular_file(&source, 16 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(source_bytes.as_slice())
        || read_bounded_regular_file(&copied_source, 16 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(source_bytes.as_slice())
    {
        return with_reason(report, "source_changed_during_scan");
    }
    if read_bounded_regular_file(&scratch.0.join("pom.xml"), 4 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(native_pom.as_bytes())
        || read_bounded_regular_file(&scratch.0.join("settings.xml"), 1024 * 1024)
            .ok()
            .as_deref()
            != Some(include_bytes!("../resources/isolated_maven_settings.xml").as_slice())
    {
        return with_reason(report, "native_configuration_changed_during_scan");
    }
    if read_bounded_regular_file(&tool, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(tool_bytes.as_slice())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(expected_repo)
    {
        return with_reason(report, "native_identity_changed_during_scan");
    }
    let Ok(xml) = read_bounded_regular_file(&scratch.0.join("target/pmd.xml"), 16 * 1024 * 1024)
    else {
        return with_reason(
            report,
            if outcome.termination == Termination::Exited(0) {
                "native_report_missing_or_invalid"
            } else {
                "native_execution_failed_and_report_missing"
            },
        );
    };
    let parsed = parse_pmd_xml(&xml, "6.15.0");
    if parsed
        .diagnostics
        .iter()
        .any(|finding| !p3c_rule_in_selected_rulesets(selected, &finding.ruleset, &finding.rule))
    {
        return with_reason(report, "native_rule_outside_selected_rulesets");
    }
    let copied_path = copied_source.to_string_lossy();
    let findings: Vec<Value> = parsed
        .diagnostics
        .iter()
        .filter(|finding| {
            finding.filename == copied_path
                && safe_label(&finding.rule)
                && safe_label(&finding.ruleset)
        })
        .map(|finding| {
            json!({"path":source_path,"line":finding.beginline,"column":finding.begincolumn,
            "rule_id":finding.rule,"ruleset":finding.ruleset,"native_priority":finding.priority,
            "rule_summary":"原生 P3C/PMD 规则报告；请查阅私有诊断与规则说明并复检"})
        })
        .collect();
    report["findings"] = json!(findings);
    if parsed.state != PmdParseState::ValidReport
        || parsed.files.iter().any(|file| file != copied_path.as_ref())
        || parsed.diagnostics.len() != findings.len()
    {
        return with_reason(report, "native_report_invalid_or_out_of_scope");
    }
    if outcome.termination != Termination::Exited(0) {
        return with_reason(report, "native_execution_failed");
    }
    if findings.is_empty() {
        report["local_status"] = json!("clean_scope_unproven");
        return with_reason(report, "clean_report_has_no_file_attestation");
    }
    if parsed.files != [copied_path.as_ref()] {
        return with_reason(report, "native_report_scope_mismatch");
    }
    report["local_status"] = json!("findings_observed_untrusted");
    with_reason(report, "native_findings_require_approved_context")
}

pub(crate) fn render_pom(selected: &[&str]) -> String {
    let template = include_str!("../resources/p3c_single_file_pom.xml");
    let start = template
        .find("      <rulesets>")
        .expect("固定探针含规则集起点");
    let end = template
        .find("      </rulesets>")
        .expect("固定探针含规则集终点")
        + "      </rulesets>".len();
    let mut result = template[..start].to_owned();
    result.push_str("      <rulesets>\n");
    for rule in selected {
        result.push_str("        <ruleset>");
        result.push_str(rule);
        result.push_str("</ruleset>\n");
    }
    result.push_str("      </rulesets>");
    result.push_str(&template[end..]);
    result
}

fn with_reason(mut report: Value, reason: &'static str) -> Value {
    report["reason"] = json!(reason);
    report
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn private_scratch() -> Result<Scratch, &'static str> {
    let temp = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "temp_root_unavailable")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let path = temp.join(format!("codeguard-p3c-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).map_err(|_| "scratch_create_failed")?;
    if fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).is_err() {
        let _ = fs::remove_dir(&path);
        return Err("scratch_permission_failed");
    }
    Ok(Scratch(path))
}
