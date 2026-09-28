//! JDK 原生 Javadoc 缺失注释的隔离单文件诊断；不签发项目质量结论。

use codeguard_adapters::{JavadocParseState, parse_javadoc_output};
use codeguard_runtime::{
    ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct Args {
    pub(crate) source: PathBuf,
    pub(crate) java_home: Option<PathBuf>,
    pub(crate) json: bool,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 运行局部 Javadoc 诊断；检查完成也只返回未评估交付的退出码 3。
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
            "Java Javadoc 单文件原生诊断：{}",
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
        println!("局部注释诊断不证明项目配置、完整源集或交付质量。");
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let Some(source) = args.first().filter(|source| !source.starts_with('-')) else {
        return Err("lint java --checker javadoc 缺少 Java 源文件路径".into());
    };
    let mut parsed = Args {
        source: PathBuf::from(source),
        java_home: None,
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
            "--java-home" if parsed.java_home.is_none() => {
                parsed.java_home = Some(PathBuf::from(value));
            }
            "--format" if matches!(value.as_str(), "human" | "json") => {
                parsed.json = value == "json";
            }
            _ => return Err(format!("lint java --checker javadoc 参数无效或重复：{key}")),
        }
        index += 1;
    }
    Ok(parsed)
}

pub(crate) fn observe(args: &Args, deadline: Instant, cancelled: &AtomicBool) -> Value {
    let source = args.source.canonicalize().ok();
    let source_path = source
        .as_deref()
        .unwrap_or(&args.source)
        .to_string_lossy()
        .into_owned();
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"java_javadoc_local_feedback",
        "operation":"lint", "language":"java", "category":"comments",
        "command_status":"incomplete", "exit_code":3, "local_status":"incomplete",
        "reason":"prerequisites_missing", "path":source_path, "source_sha256":null,
        "javadoc_tool_sha256":null, "java_runtime_sha256":null, "jdk_release_sha256":null,
        "checker_identity":"unverified", "coverage_proven":false, "findings":[],
        "authority":"local_unverified", "delivery_decision":"not_evaluated",
        "next_actions":["verify_project_javadoc_configuration_and_source_scope","bind_approved_tool_and_quality_policy"]
    });
    let (Some(source), Some(java_home)) = (source, args.java_home.as_ref()) else {
        return report;
    };
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
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
    if std::str::from_utf8(&source_bytes).is_err() {
        return with_reason(report, "source_encoding_unverified");
    }
    report["source_sha256"] = json!(digest(&source_bytes));
    let Ok(java_home) = java_home.canonicalize() else {
        return with_reason(report, "java_home_unavailable");
    };
    let (Ok(javadoc), Ok(java)) = (
        java_home.join("bin/javadoc").canonicalize(),
        java_home.join("bin/java").canonicalize(),
    ) else {
        return with_reason(report, "jdk_tool_unavailable");
    };
    let (Ok(javadoc_bytes), Ok(java_bytes)) = (
        read_bounded_regular_file(&javadoc, 128 * 1024 * 1024),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
    ) else {
        return with_reason(report, "jdk_tool_unavailable");
    };
    let Ok(release_bytes) = read_bounded_regular_file(&java_home.join("release"), 64 * 1024) else {
        return with_reason(report, "jdk_version_unverified");
    };
    if !std::str::from_utf8(&release_bytes)
        .ok()
        .is_some_and(|text| {
            text.lines().any(|line| {
                line.strip_prefix("JAVA_VERSION=\"21.")
                    .is_some_and(|version| version.ends_with('"') && version.len() < 32)
            })
        })
    {
        return with_reason(report, "jdk_version_unverified");
    }
    report["javadoc_tool_sha256"] = json!(digest(&javadoc_bytes));
    report["java_runtime_sha256"] = json!(digest(&java_bytes));
    report["jdk_release_sha256"] = json!(digest(&release_bytes));
    let (Some(parent), Some(name)) = (source.parent(), source.file_name()) else {
        return with_reason(report, "source_unavailable");
    };
    let relative = PathBuf::from(name);
    let Ok(snapshot) = SourceSnapshot::capture(
        parent,
        [relative.clone()],
        1,
        16 * 1024 * 1024,
        16 * 1024 * 1024,
    ) else {
        return with_reason(report, "source_snapshot_unavailable");
    };
    if snapshot.files().get(&relative) != Some(&source_bytes) {
        return with_reason(report, "source_changed_before_scan");
    }
    let Ok(scratch) = private_scratch() else {
        return with_reason(report, "private_workspace_unavailable");
    };
    let source_dir = scratch.0.join("src");
    let evidence_dir = scratch.0.join("evidence");
    if snapshot.materialize_new(&source_dir).is_err()
        || fs::create_dir(&evidence_dir).is_err()
        || fs::set_permissions(&evidence_dir, fs::Permissions::from_mode(0o700)).is_err()
    {
        return with_reason(report, "private_workspace_unavailable");
    }
    let copied = source_dir.join(name);
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_os_string(),
    );
    env.insert(
        OsString::from("PATH"),
        OsString::from(format!("{}:/usr/bin:/bin", java_home.join("bin").display())),
    );
    env.insert(OsString::from("LANG"), OsString::from("C"));
    let spec = ProcessSpec {
        executable: javadoc.clone(),
        args: vec![
            "-J-Duser.language=en".into(),
            "-J-Duser.country=US".into(),
            "-encoding".into(),
            "UTF-8".into(),
            "-Xdoclint:missing".into(),
            "-quiet".into(),
            "-d".into(),
            scratch.0.join("docs").into_os_string(),
            copied.clone().into_os_string(),
        ],
        cwd: scratch.0.clone(),
        env,
        stdin: None,
        deadline,
        output_limit_bytes: 1024 * 1024,
    };
    let outcome = match run_process_recorded(&spec, cancelled, &evidence_dir, "javadoc.log") {
        Ok(outcome) => outcome,
        Err(_) => return with_reason(report, "native_process_incomplete"),
    };
    if snapshot.verify_unchanged(&source_dir).ok() != Some(true) {
        return with_reason(report, "source_changed_during_scan");
    }
    if read_bounded_regular_file(&javadoc, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(javadoc_bytes.as_slice())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || read_bounded_regular_file(&java_home.join("release"), 64 * 1024)
            .ok()
            .as_deref()
            != Some(release_bytes.as_slice())
    {
        return with_reason(report, "jdk_identity_changed_during_scan");
    }
    if outcome.termination != Termination::Exited(0) {
        return with_reason(report, "native_execution_incomplete");
    }
    if !outcome.stdout.is_empty() {
        return with_reason(report, "native_output_unrecognized");
    }
    let copied_path = copied.to_string_lossy();
    let parsed = parse_javadoc_output(&outcome.stderr, &copied_path, &source_bytes);
    if parsed.state != JavadocParseState::ValidDiagnostics {
        return with_reason(
            report,
            parsed.reason.unwrap_or("native_output_unrecognized"),
        );
    }
    if read_bounded_regular_file(&scratch.0.join("docs/index.html"), 16 * 1024 * 1024).is_err() {
        return with_reason(report, "native_output_missing");
    }
    let mut findings = Vec::new();
    for diagnostic in parsed.diagnostics {
        let Some(line) = source_bytes
            .split(|byte| *byte == b'\n')
            .nth((diagnostic.line - 1) as usize)
        else {
            return with_reason(report, "native_location_invalid");
        };
        if diagnostic.column as usize > line.len().saturating_add(1) {
            return with_reason(report, "native_location_invalid");
        }
        findings.push(json!({
            "path":source_path, "line":diagnostic.line, "column":diagnostic.column,
            "rule_id":diagnostic.rule_id,
            "rule_summary":"JDK Javadoc 缺失注释或标签；核对原始源码后复检"
        }));
    }
    report["findings"] = json!(findings);
    if report["findings"].as_array().is_some_and(Vec::is_empty) {
        report["local_status"] = json!("clean_scope_unproven");
        with_reason(report, "single_file_no_missing_doc_diagnostic")
    } else {
        report["local_status"] = json!("findings_observed_untrusted");
        with_reason(report, "native_findings_require_approved_context")
    }
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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let path = temp.join(format!("codeguard-javadoc-{}-{nanos}", std::process::id()));
    fs::create_dir(&path).map_err(|_| "scratch_create_failed")?;
    if fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).is_err() {
        let _ = fs::remove_dir(&path);
        return Err("scratch_permission_failed");
    }
    Ok(Scratch(path))
}
