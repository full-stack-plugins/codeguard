//! 原生 Cargo/check 类型检查观察；完整项目策略与修复任务仍独立核验。

use crate::cargo_build_repair_brief::cargo_build_repair_brief;
use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::discovery::discover;
use crate::doctor_scratch::DoctorScratch;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::cargo_build_finding_record;
use codeguard_adapters::{legacy_registry, parse_cargo_build_json};
use codeguard_runtime::{
    NativeObservation, ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file,
    run_process,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Arguments {
    root: PathBuf,
    tool: Option<PathBuf>,
    json: bool,
    timeout: Option<u64>,
}

/// 执行 build rust 的局部构建观察；参数为该入口剩余 argv，不授予完整门禁或任务关闭。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match parse_args(args) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let timeout = select_check_timeout(arguments.timeout)
        .and_then(|(value, source)| resolve_project_default(&arguments.root, value, source));
    let (timeout, source) = match timeout {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let mut report = empty_report(timeout, source);
    if let Some(root) = arguments
        .root
        .canonicalize()
        .ok()
        .filter(|root| root.is_dir())
    {
        report = observe_for_verification(
            &root,
            arguments.tool.as_deref(),
            timeout,
            source,
            deadline,
            &AtomicBool::new(false),
        );
        persist_and_sync(&root, &mut report);
    }
    let exit = if report["reason"] == "request_cancelled" {
        130
    } else {
        3
    };
    if exit == 130 {
        report["exit_code"] = json!(130);
        report["command_status"] = json!("cancelled");
    }
    if arguments.json {
        println!("{report}");
    } else {
        println!(
            "Rust 构建检查：未完成；原生观察 {}，原因 {}",
            report["local_scan_complete"], report["reason"]
        );
        for finding in report["findings"].as_array().into_iter().flatten() {
            println!(
                "原生规则 {}：{}:{}；按规则依据修复编译问题后用原工具复检",
                finding["rule_id"], finding["path"], finding["line"]
            );
            println!("规则依据：{}", finding["repair_brief"]["rule_basis"]);
            for step in finding["repair_brief"]["steps"]
                .as_array()
                .into_iter()
                .flatten()
            {
                println!("下一步：{}", step.as_str().unwrap_or(""));
            }
        }
        println!(
            "任务同步：{}；库目标局部探针不代表原配置、全部构建组合或已修复；正式关闭与复发重开尚未接通。",
            report["backlog_status"]
        );
    }
    ExitCode::from(exit)
}

fn empty_report(timeout: u64, source: &str) -> Value {
    let report = json!({
        "schema_version":"0.2.0","report_type":"rust_build_local_observation","operation":"build","language":"rust",
        "workspace_binding":"uninitialized","workspace_id":null,"run_id":format!("cargo-build-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_or(0,|value|value.as_nanos())),"backlog_sync":null,
        "command_status":"incomplete","exit_code":3,"delivery_decision":"not_evaluated","authority":"local_unverified",
        "checker_id":"rust.cargo_check","scope":"selected_root_all_targets_default_features",
        "rule_selection":"explicit_probe_not_approved_project_policy","local_scan_complete":false,"coverage_proven":false,
        "build_level":"type_check","test_execution":false,"build_success":null,"reason":"project_root_unavailable","tool_sha256":null,"manifest_sha256":null,"lock_sha256":null,"findings":[],
        "backlog_status":"not_integrated","execution_budget":budget_record(timeout,source),
        "next_actions":["核对原生项目构建规则与目标范围；局部零诊断不证明修复或批准白名单","原工具复检后仍须完整交付检查"],
        "recheck_argv":["cargo","check","--locked","--offline","--all-targets","--message-format=json"]
    });
    report
}

/// 使用调用方截止时间生成原生构建观察，不同步或关闭任务。
/// 参数为项目、工具、预算来源与取消标记；返回未批准的局部报告。
pub(crate) fn observe_for_verification(
    root: &Path,
    tool: Option<&Path>,
    timeout: u64,
    source: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = empty_report(timeout, source);
    match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => {
                report["workspace_binding"] = json!("bound");
                report["workspace_id"] = json!(id);
            }
            None => report["workspace_binding"] = json!("legacy_unbound"),
        },
        Ok(None) => {}
        Err(_) => report["workspace_binding"] = json!("invalid"),
    }
    observe(root, tool, deadline, &mut report, cancelled);
    if report["workspace_binding"] == "bound" {
        report["backlog_status"] = json!("queued");
    }

    if report["reason"] == "request_cancelled" {
        report["exit_code"] = json!(130);
        report["command_status"] = json!("cancelled");
    }
    report
}

fn observe(
    root: &Path,
    tool: Option<&Path>,
    deadline: Instant,
    report: &mut Value,
    cancelled: &AtomicBool,
) {
    report["reason"] = json!("cargo_tool_not_selected");
    let selected_tool = crate::cargo_tool_selection::resolve_cargo_tool(tool);
    let Some(tool) = selected_tool.as_deref() else {
        return;
    };
    let Ok(resolved_tool) = tool.canonicalize() else {
        report["reason"] = json!("cargo_tool_unavailable");
        return;
    };
    let Ok(tool_before) = read_bounded_regular_file(&resolved_tool, 128 * 1024 * 1024) else {
        report["reason"] = json!("cargo_tool_unavailable");
        return;
    };
    report["tool_sha256"] = json!(format!("{:x}", Sha256::digest(&tool_before)));
    let Ok(registry) = legacy_registry() else {
        report["reason"] = json!("language_registry_invalid");
        return;
    };
    let discovery = discover(root, &registry, &NativeObservation);
    if !discovery.observation_complete {
        report["reason"] = json!("source_discovery_incomplete");
        return;
    }
    let Some(language) = discovery.languages.get("rust") else {
        report["reason"] = json!("rust_source_scope_missing");
        return;
    };
    let mut paths: Vec<PathBuf> = language.source_files.iter().map(PathBuf::from).collect();
    paths.extend([PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")]);
    let Ok(snapshot) =
        SourceSnapshot::capture(root, paths, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024)
    else {
        report["reason"] = json!("locked_cargo_inputs_unavailable");
        return;
    };
    for (file, field) in [
        ("Cargo.toml", "manifest_sha256"),
        ("Cargo.lock", "lock_sha256"),
    ] {
        report[field] = json!(format!(
            "{:x}",
            Sha256::digest(
                snapshot
                    .files()
                    .get(Path::new(file))
                    .expect("已捕获Cargo输入")
            )
        ));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let Some(scratch) =
        DoctorScratch::create(&format!("cargo-build-{}-{nonce}", std::process::id()))
    else {
        report["reason"] = json!("private_workspace_unavailable");
        return;
    };
    let mut environment = BTreeMap::new();
    for name in [
        "PATH",
        "HOME",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
    ] {
        if let Some(value) = std::env::var_os(name) {
            environment.insert(OsString::from(name), value);
        }
    }
    environment.insert(
        OsString::from("CARGO_TARGET_DIR"),
        scratch.path().as_os_str().to_os_string(),
    );
    environment.insert(OsString::from("CARGO_NET_OFFLINE"), OsString::from("true"));
    // Cargo 的离线选项不约束 rustup；检查不得自动安装缺失工具链。
    environment.insert(OsString::from("RUSTUP_AUTO_INSTALL"), OsString::from("0"));
    let result = run_process(
        &ProcessSpec {
            // 保留 Cargo 代理入口名；直接运行解析后的 rustup 会改变命令语义。
            executable: tool.to_path_buf(),
            args: [
                "check",
                "--locked",
                "--offline",
                "--all-targets",
                "--message-format=json",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
            cwd: root.to_path_buf(),
            env: environment,
            stdin: None,
            deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        },
        cancelled,
    );
    let parsed = parse_cargo_build_json(
        &result.stdout,
        match result.termination {
            Termination::Exited(code) => code,
            _ => -1,
        },
    );
    report["build_success"] = json!(parsed.build_success);
    let mut outside = false;
    let mut seen = BTreeSet::new();
    let mut range_issue = None;
    let mut findings: Vec<Value> = parsed
        .diagnostics
        .iter()
        .filter_map(|finding| {
            let relative = Path::new(&finding.path);
            if !language.source_files.contains(&finding.path)
                || finding.manifest_path != root.join("Cargo.toml").to_string_lossy()
                || !language
                    .source_files
                    .iter()
                    .any(|path| root.join(path).to_string_lossy() == finding.target_source)
            {
                outside = true;
                return None;
            }
            let bytes = snapshot.files().get(relative)?;
            let Some(target) = Path::new(&finding.target_source)
                .strip_prefix(root)
                .ok()
                .and_then(|path| path.to_str())
            else {
                outside = true;
                return None;
            };
            let mut record = match cargo_build_finding_record(finding, bytes, target) {
                Ok(record) => record,
                Err(reason) => {
                    range_issue.get_or_insert(reason);
                    return None;
                }
            };
            let Some(target_bytes) = snapshot.files().get(Path::new(target)) else {
                outside = true;
                return None;
            };
            record["target_sha256"] = json!(format!("{:x}", Sha256::digest(target_bytes)));
            let exact = (
                record["finding_fingerprint"]
                    .as_str()
                    .expect("观察指纹")
                    .to_owned(),
                finding.byte_start,
                finding.byte_end,
                finding.code.clone(),
                finding.target_kinds.clone(),
                finding.line,
                finding.column,
            );
            if !seen.insert(exact) {
                return None;
            }
            Some(record)
        })
        .collect();
    let mut counts = BTreeMap::<String, usize>::new();
    for finding in &findings {
        *counts
            .entry(
                finding["finding_fingerprint"]
                    .as_str()
                    .expect("观察指纹")
                    .to_owned(),
            )
            .or_default() += 1;
    }
    let mut ambiguous = false;
    for finding in &mut findings {
        if counts[finding["finding_fingerprint"].as_str().expect("观察指纹")] > 1 {
            ambiguous = true;
            finding["finding_id"] = Value::Null;
            finding["identity_status"] = json!("ambiguous");
        }
    }
    report["findings"] = json!(findings);
    let after = discover(root, &registry, &NativeObservation);
    let unchanged = snapshot.verify_unchanged(root).unwrap_or(false)
        && after.observation_complete
        && after.languages.get("rust").map(|value| &value.source_files)
            == Some(&language.source_files);
    // 用户取消优先于复核失败；不能把一次取消降级为普通未完成退出。
    let reason = if result.termination == Termination::Cancelled {
        "request_cancelled"
    } else if !unchanged {
        "inputs_changed_during_scan"
    } else if !tool
        .canonicalize()
        .is_ok_and(|after| after == resolved_tool)
        || !read_bounded_regular_file(&resolved_tool, 128 * 1024 * 1024)
            .is_ok_and(|bytes| bytes == tool_before)
    {
        "tool_changed_during_scan"
    } else if outside {
        "native_target_outside_observed_scope"
    } else if let Some(reason) = range_issue {
        reason
    } else if ambiguous {
        "native_finding_identity_ambiguous"
    } else {
        match result.termination {
            Termination::Exited(0 | 101) => parsed.issue.unwrap_or("native_observed_unverified"),
            Termination::Exited(_) => parsed.issue.unwrap_or("native_process_failed"),
            Termination::Cancelled => "request_cancelled",
            Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
            Termination::OutputLimit => "native_output_limit_exceeded",
            _ => "native_execution_failed",
        }
    };
    report["reason"] = json!(reason);
    report["local_scan_complete"] = json!(reason == "native_observed_unverified");
    let recheck = report["recheck_argv"].clone();
    for finding in report["findings"].as_array_mut().into_iter().flatten() {
        finding["repair_brief"] = cargo_build_repair_brief(
            finding,
            reason == "native_observed_unverified",
            reason,
            &recheck,
        );
    }
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut tool = None;
    let mut format = None;
    let mut timeout = None;
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        let (key, value) = if let Some((key, value)) = current.split_once('=') {
            (key, Some(value))
        } else if matches!(current.as_str(), "--cargo-tool" | "--format" | "--timeout") {
            index += 1;
            (
                current.as_str(),
                Some(args.get(index).ok_or("参数缺少值")?.as_str()),
            )
        } else {
            (current.as_str(), None)
        };
        match (key, value) {
            ("--cargo-tool", Some(value)) if tool.is_none() && Path::new(value).is_absolute() => {
                tool = Some(PathBuf::from(value))
            }
            ("--format", Some(value)) if format.is_none() && matches!(value, "json" | "human") => {
                format = Some(value)
            }
            ("--timeout", Some(value)) if timeout.is_none() => {
                timeout = Some(parse_check_timeout(value)?)
            }
            (_, None) if !current.is_empty() && !current.starts_with('-') && root.is_none() => {
                root = Some(PathBuf::from(current))
            }
            _ => return Err("build rust 参数非法；工具须为绝对路径，格式仅human/json".into()),
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        tool,
        json: format == Some("json"),
        timeout,
    })
}

/// 保存并同步构建局部报告；成功同步不授权任务关闭。
pub(crate) fn persist_and_sync(root: &Path, report: &mut Value) {
    if report["workspace_binding"] == "bound" {
        report["backlog_status"] = json!("queued");
        match save_local_report(root, report).and_then(|()| sync_local_workspace(root)) {
            Ok(summary) => {
                report["backlog_status"] = json!(if summary.failed_reports == 0 {
                    "synced"
                } else {
                    "sync_incomplete"
                });
                report["backlog_sync"] = json!({"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"failed_reports":summary.failed_reports});
            }
            Err(reason) => {
                report["backlog_status"] = json!("failed");
                report["backlog_sync"] = json!({"reason":reason});
            }
        }
    }
}
