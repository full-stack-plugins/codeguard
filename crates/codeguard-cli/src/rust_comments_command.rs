//! 原生 Cargo/rustdoc 库目标观察；完整项目策略与修复任务仍独立核验。

use crate::check_budget::{budget_record, parse_check_timeout};
use crate::discovery::discover;
use crate::doctor_scratch::DoctorScratch;
use crate::rustdoc_repair_brief::rustdoc_repair_brief;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::rustdoc_finding_record;
use codeguard_adapters::{legacy_registry, parse_cargo_rustdoc_json};
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
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Rust 注释入口的已校验参数，供统一编排与原生观察共用。
pub(crate) struct Arguments {
    pub root: PathBuf,
    pub tool: Option<PathBuf>,
    pub json: bool,
    pub timeout: Option<u64>,
}

/// 执行 comments rust 的局部文档观察；参数为该入口剩余 argv，不授予完整门禁或任务关闭。
pub fn run(args: &[String]) -> ExitCode {
    crate::rust_documentation_command::run(args)
}

/// 构造原 rustdoc 未完成报告；参数为时间预算与来源，不授予检查资格。
pub(crate) fn empty_report(timeout: u64, source: &str) -> Value {
    let report = json!({
        "schema_version":"0.4.0","report_type":"rustdoc_local_observation","operation":"comments","language":"rust",
        "workspace_binding":"uninitialized","workspace_id":null,"run_id":format!("rustdoc-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_or(0,|value|value.as_nanos())),"backlog_sync":null,
        "command_status":"incomplete","exit_code":3,"delivery_decision":"not_evaluated","authority":"local_unverified",
        "checker_id":"rust.cargo_rustdoc","scope":"selected_root_library_default_features",
        "rule_selection":"explicit_probe_not_approved_project_policy","local_scan_complete":false,"coverage_proven":false,
        "reason":"project_root_unavailable","tool_sha256":null,"manifest_sha256":null,"lock_sha256":null,"findings":[],
        "backlog_status":"not_integrated","execution_budget":budget_record(timeout,source),
        "next_actions":["核对原生项目文档规则与目标范围；局部零诊断不证明修复或批准白名单","原工具复检后仍须完整交付检查"],
        "recheck_argv":["cargo","rustdoc","--lib","--locked","--offline","--message-format=json","--","--warn","missing_docs","--warn","rustdoc::broken_intra_doc_links"]
    });
    report
}

/// 使用调用方截止时间生成原生文档观察，不同步或关闭任务。
/// 参数为项目、工具、预算来源与是否使用原生强制告警；返回未批准的局部报告。
pub(crate) fn observe_for_verification(
    root: &Path,
    tool: Option<&Path>,
    timeout: u64,
    source: &str,
    deadline: Instant,
    force_warn: bool,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = empty_report(timeout, source);
    if force_warn {
        report["recheck_argv"][7] = json!("--force-warn");
        report["recheck_argv"][9] = json!("--force-warn");
    }
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
    observe(root, tool, deadline, &mut report, force_warn, cancelled);

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
    force_warn: bool,
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
    let Some(scratch) = DoctorScratch::create(&format!("rustdoc-{}-{nonce}", std::process::id()))
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
                "rustdoc",
                "--lib",
                "--locked",
                "--offline",
                "--message-format=json",
                "--",
                if force_warn { "--force-warn" } else { "--warn" },
                "missing_docs",
                if force_warn { "--force-warn" } else { "--warn" },
                "rustdoc::broken_intra_doc_links",
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
    let parsed = parse_cargo_rustdoc_json(&result.stdout);
    let mut outside = false;
    let mut seen = BTreeSet::new();
    let mut range_issue = None;
    let mut findings: Vec<Value> = parsed
        .findings
        .iter()
        .filter_map(|finding| {
            let relative = Path::new(&finding.path);
            if !language.source_files.contains(&finding.path)
                || finding.manifest_path != root.join("Cargo.toml").to_string_lossy()
                || !language
                    .source_files
                    .iter()
                    .any(|path| root.join(path).to_string_lossy() == finding.target_source)
                || finding.target_kinds != ["lib"]
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
            let record = match rustdoc_finding_record(finding, bytes, target) {
                Ok(record) => record,
                Err(reason) => {
                    range_issue.get_or_insert(reason);
                    return None;
                }
            };
            let exact = (
                record["finding_fingerprint"]
                    .as_str()
                    .expect("观察指纹")
                    .to_owned(),
                finding.byte_start,
                finding.byte_end,
                finding.level.clone(),
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
            Termination::Exited(0) => parsed.issue.unwrap_or("native_observed_unverified"),
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
        finding["repair_brief"] = rustdoc_repair_brief(
            finding,
            reason == "native_observed_unverified",
            reason,
            &recheck,
        );
    }
}

/// 解析语种后 argv，返回路径/工具/预算参数；非法参数在原生执行前拒绝。
pub(crate) fn parse_args(args: &[String]) -> Result<Arguments, String> {
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
            _ => return Err("comments rust 参数非法；工具须为绝对路径，格式仅human/json".into()),
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

/// 保存并同步文档局部观察；不改变原生检查结果或授权关闭。
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
