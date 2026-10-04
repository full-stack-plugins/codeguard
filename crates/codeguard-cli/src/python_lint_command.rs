//! Python lint 的临时 CLI 接线：配置探测、Ruff 原生执行与对话摘要。

use crate::check_budget::{
    budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
};
use crate::discovery::discover;
use crate::next_command::read_local_brief;
use crate::python_lint_scan::{
    PythonLintScanRequest, python_lint_feedback, scan_python_lint, select_checker,
};
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_adapters::{bundled_ruff_rulepack, legacy_registry};
use codeguard_runtime::{
    NativeObservation, ProcessSpec, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Arguments {
    root: PathBuf,
    json: bool,
    ruff_tool: Option<PathBuf>,
    selected_paths: Vec<String>,
    timeout_ms: u64,
    timeout_source: &'static str,
}

struct PrivateScratch(PathBuf);

impl Drop for PrivateScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct ToolSetup {
    path: Option<PathBuf>,
    sha256: [u8; 32],
    version: String,
    unavailable_reason: &'static str,
    scratch: Option<PrivateScratch>,
}

/// 运行配置感知的局部 Python lint，并将本次反馈直接输出到 CLI。
pub fn run(args: &[String]) -> ExitCode {
    let mut parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("项目路径不可读取");
            return ExitCode::from(3);
        }
    };
    match resolve_project_default(&root, parsed.timeout_ms, parsed.timeout_source) {
        Ok((timeout_ms, source)) => {
            parsed.timeout_ms = timeout_ms;
            parsed.timeout_source = source;
        }
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    }
    let deadline = started + Duration::from_millis(parsed.timeout_ms);
    let mut feedback = match if parsed.selected_paths.is_empty() {
        scan_and_sync_report_with_deadline(
            &root,
            parsed.ruff_tool.as_deref(),
            deadline,
            &AtomicBool::new(false),
        )
    } else {
        scan_selected_report_with_deadline(
            &root,
            parsed.ruff_tool.as_deref(),
            &parsed.selected_paths,
            deadline,
            &AtomicBool::new(false),
        )
    } {
        Ok(feedback) => feedback,
        Err(_) => {
            eprintln!("内置语言清单损坏");
            return ExitCode::from(4);
        }
    };
    annotate_conversation_budget(&mut feedback, parsed.timeout_ms, parsed.timeout_source);
    feedback["schema_version"] = Value::String("0.13.0".into());
    if parsed.selected_paths.is_empty() {
        feedback["scan_scope"] = Value::String("discovered_python".into());
        feedback["requested_paths"] = Value::Null;
    }
    let cancellation_observed = codeguard_runtime::sigint_cancellation_requested()
        || feedback["incomplete_reasons"]
            .as_array()
            .is_some_and(|reasons| reasons.iter().any(|reason| reason == "request_cancelled"));
    #[cfg(feature = "wasm-precheck")]
    let mut request_cancelled = cancellation_observed;
    #[cfg(not(feature = "wasm-precheck"))]
    let request_cancelled = cancellation_observed;
    if request_cancelled {
        feedback["command_status"] = Value::String("cancelled".into());
        feedback["exit_code"] = Value::from(130);
        if let Some(reasons) = feedback["incomplete_reasons"].as_array_mut() {
            if !reasons.iter().any(|reason| reason == "request_cancelled") {
                reasons.push(Value::String("request_cancelled".into()));
            }
        }
    }
    #[cfg(feature = "wasm-precheck")]
    if !request_cancelled {
        if let Some(precheck) = crate::python_syntax_precheck::observe(&root, &feedback, deadline) {
            let has_structure = precheck["structural_observations"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty());
            feedback["schema_version"] =
                Value::String(if has_structure { "0.16.0" } else { "0.14.0" }.into());
            feedback["scope"] = Value::String("local_native_and_candidate_syntax_scan".into());
            feedback["execution_budget"]["enforcement"] =
                Value::String("native_and_bounded_syntax_worker".into());
            let native_reasons: std::collections::BTreeSet<&str> = feedback["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|file| file["reason"].as_str())
                .collect();
            let native_reason = if native_reasons.len() == 1 {
                native_reasons
                    .iter()
                    .next()
                    .copied()
                    .unwrap_or("native_context_unavailable")
            } else {
                "multiple_native_incomplete_reasons"
            };
            feedback["native"] = serde_json::json!({"status":"incomplete","reason":native_reason});
            feedback["setup"] = serde_json::json!({"requirement":"required","reason":"native_confirmation_needed","task_id":null});
            feedback["next_action"] = Value::String(
                if has_structure {
                    "核对 codeguard.python.required_suite 候选结构规则观察，恢复项目原生检查器及配置并复检；结构观察不是原生违规或完成证据"
                } else if precheck["observations"].as_array().is_some_and(|rows| !rows.is_empty()) {
                    "核对 Python 疑似语法位置，确认项目要求的原生检查器及配置，再以适用的 Python 原生语法能力复检；勿凭候选初检修改源码或关闭任务"
                } else {
                    "候选 Python grammar 版本尚未验收；确认项目要求的原生检查器及配置后复检，不把零恢复节点当作通过"
                }.into(),
            );
            if feedback["workspace_binding"] == "bound" {
                let task = crate::python_syntax_confirmation::persist(&root, &precheck, deadline);
                feedback["schema_version"] =
                    Value::String(if has_structure { "0.17.0" } else { "0.15.0" }.into());
                match task {
                    Ok(id) => {
                        feedback["setup"]["task_id"] = Value::String(id);
                        feedback["task_persistence"] =
                            serde_json::json!({"status":"synced_partial","reason":null});
                    }
                    Err(reason) => {
                        feedback["task_persistence"] =
                            serde_json::json!({"status":"unavailable","reason":reason});
                    }
                }
            }
            feedback["syntax_precheck"] = precheck;
        }
    }
    #[cfg(feature = "wasm-precheck")]
    if !request_cancelled && codeguard_runtime::sigint_cancellation_requested() {
        request_cancelled = true;
        feedback["command_status"] = Value::String("cancelled".into());
        feedback["exit_code"] = Value::from(130);
        if let Some(reasons) = feedback["incomplete_reasons"].as_array_mut() {
            if !reasons.iter().any(|reason| reason == "request_cancelled") {
                reasons.push(Value::String("request_cancelled".into()));
            }
        }
    }
    if parsed.json {
        println!("{feedback}");
    } else {
        print_human(&feedback);
    }
    // 工具身份和策略批准、其它 Python 检查族尚未接入，不能签发质量通过。
    ExitCode::from(if request_cancelled { 130 } else { 3 })
}

/// 扫描并同步本轮 Python lint，保留原生反馈与 backlog/下一步的局部状态。
///
/// 参数 `root` 为规范化项目根；返回值不代表可信工具、策略或交付门禁已通过。
pub fn scan_and_sync_report(root: &Path, ruff_tool: Option<&Path>) -> Result<Value, &'static str> {
    scan_and_sync_report_with_deadline(
        root,
        ruff_tool,
        Instant::now() + Duration::from_secs(120),
        &AtomicBool::new(false),
    )
}

/// 使用调用方提供的整次操作截止时间扫描并同步，原生探测和逐文件检查不得重置预算。
pub fn scan_and_sync_report_with_deadline(
    root: &Path,
    ruff_tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    let mut feedback = scan_local_report_with_deadline(root, ruff_tool, deadline, cancelled)?;
    let (backlog_status, backlog_summary) = match save_local_report(root, &feedback) {
        Ok(()) if feedback["workspace_binding"] == "bound" => match sync_local_workspace(root) {
            Ok(summary) => (
                if summary.failed_reports == 0 {
                    "synced_partial"
                } else {
                    "backlog_update_failed"
                },
                serde_json::json!({"new_findings":summary.new_findings,
                    "new_blockers":summary.new_blockers,
                    "imported_reports":summary.imported_reports,
                    "historical_findings":summary.historical_findings,
                    "failed_reports":summary.failed_reports}),
            ),
            Err(_) => ("backlog_update_failed", Value::Null),
        },
        Ok(()) => ("not_initialized", Value::Null),
        Err(_) => ("backlog_update_failed", Value::Null),
    };
    let document = feedback.as_object_mut().expect("反馈根为对象");
    document.insert(
        "backlog_status".into(),
        Value::String(backlog_status.into()),
    );
    document.insert("backlog_sync".into(), backlog_summary);
    let (brief_status, brief_reason, next) = if backlog_status == "synced_partial" {
        match read_local_brief(root) {
            Ok(next) => ("available", Value::Null, next),
            Err(reason) => ("unavailable", Value::String(reason.into()), Value::Null),
        }
    } else {
        (
            "unavailable",
            Value::String(backlog_status.into()),
            Value::Null,
        )
    };
    let document = feedback.as_object_mut().expect("反馈根为对象");
    document.insert("schema_version".into(), Value::String("0.10.0".into()));
    document.insert(
        "repair_brief_status".into(),
        Value::String(brief_status.into()),
    );
    document.insert("repair_brief_reason".into(), brief_reason);
    document.insert("next".into(), next);
    Ok(feedback)
}

/// 为公开对话反馈加入预算来源；工作区内保存的原始 0.9 扫描报告保持独立。
pub fn annotate_conversation_budget(feedback: &mut Value, timeout_ms: u64, source: &str) {
    let document = feedback.as_object_mut().expect("反馈根为对象");
    document.insert("schema_version".into(), Value::String("0.12.0".into()));
    document.insert("execution_budget".into(), budget_record(timeout_ms, source));
}

/// 编辑快反馈仅观察 `selected_paths`，不把局部报告作为完整工作区报告导入工作台。
/// `root` 是规范化项目根，`deadline` 和 `cancelled` 约束本轮原生执行；返回局部报告或清单错误。
pub(crate) fn scan_selected_report_with_deadline(
    root: &Path,
    ruff_tool: Option<&Path>,
    selected_paths: &[String],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    let mut feedback = scan_local_report_scoped_with_deadline(
        root,
        ruff_tool,
        Some(selected_paths),
        deadline,
        cancelled,
    )?;
    let document = feedback.as_object_mut().expect("反馈根为对象");
    document.insert("scan_scope".into(), Value::String("selected_files".into()));
    document.insert("requested_paths".into(), serde_json::json!(selected_paths));
    document.insert(
        "backlog_status".into(),
        Value::String("not_synced_scoped".into()),
    );
    document.insert("backlog_sync".into(), Value::Null);
    document.insert(
        "repair_brief_status".into(),
        Value::String("unavailable".into()),
    );
    document.insert(
        "repair_brief_reason".into(),
        Value::String("not_synced_scoped".into()),
    );
    document.insert("next".into(), Value::Null);
    Ok(feedback)
}

/// 对项目运行同一条局部原生 Ruff 检查链，返回可供持久同步的 0.9 报告。
///
/// 参数 `root` 是规范化的项目根，`ruff_tool` 是可选的绝对可执行路径；
/// 返回的报告尚未经过可信工具/策略批准，不是交付门禁结果。
pub fn scan_local_report(root: &Path, ruff_tool: Option<&Path>) -> Result<Value, &'static str> {
    scan_local_report_with_deadline(
        root,
        ruff_tool,
        Instant::now() + Duration::from_secs(120),
        &AtomicBool::new(false),
    )
}

/// 沿用调用方的截止时间生成局部原生报告，不为工具探测或每个文件重新计时。
pub fn scan_local_report_with_deadline(
    root: &Path,
    ruff_tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    scan_local_report_scoped_with_deadline(root, ruff_tool, None, deadline, cancelled)
}

fn scan_local_report_scoped_with_deadline(
    root: &Path,
    ruff_tool: Option<&Path>,
    selected_paths: Option<&[String]>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    let registry = legacy_registry().map_err(|_| "registry_invalid")?;
    let discovery = match selected_paths {
        Some(paths) => crate::python_selected_discovery::discover_selected(
            root,
            paths,
            &registry,
            &NativeObservation,
            deadline,
        ),
        None => discover(root, &registry, &NativeObservation),
    };
    let run_id = run_id();
    let configured = selected_paths.map_or_else(
        || {
            discovery.checker_configurations.iter().any(|checker| {
                checker.checker_id == "python.ruff" && checker.configuration == "configured"
            })
        },
        |paths| {
            paths.iter().any(|path| {
                select_checker(&discovery, path)
                    .is_some_and(|checker| checker.configuration == "configured")
            })
        },
    );
    let tool = if configured {
        prepare_tool(root, ruff_tool, deadline, &run_id, cancelled)
    } else {
        ToolSetup {
            path: None,
            sha256: [0; 32],
            version: String::new(),
            unavailable_reason: "ruff_tool_not_needed",
            scratch: None,
        }
    };
    let evidence_dir = tool.scratch.as_ref().map_or_else(
        || root.join(".codeguard-absent-evidence"),
        |scratch| scratch.0.clone(),
    );
    let native_tool_version = tool.path.as_ref().map(|_| tool.version.clone());
    let scan = scan_python_lint(
        &PythonLintScanRequest {
            root,
            discovery: &discovery,
            selected_paths,
            tool: tool.path,
            tool_unavailable_reason: tool.unavailable_reason,
            expected_tool_sha256: tool.sha256,
            expected_version: tool.version,
            evidence_dir,
            run_id: run_id.clone(),
            deadline,
        },
        cancelled,
    );
    let mut feedback = python_lint_feedback(&discovery, &scan);
    annotate_rulepack(&mut feedback, native_tool_version.as_deref());
    let has_native_findings = feedback["files"]
        .as_array()
        .is_some_and(|files| files.iter().any(|file| file["run_status"] == "findings"));
    let adapter_sha256 = if has_native_findings
        && !cancelled.load(std::sync::atomic::Ordering::Relaxed)
        && Instant::now() < deadline
    {
        let digest = current_adapter_sha256();
        if Instant::now() < deadline && !cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            digest.map_or(Value::Null, Value::String)
        } else {
            Value::Null
        }
    } else {
        Value::Null
    };
    let document = feedback.as_object_mut().expect("反馈根为对象");
    let (workspace_binding, workspace_id) = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => ("bound", Some(id.to_owned())),
            None => ("legacy_unbound", None),
        },
        Ok(None) => ("uninitialized", None),
        Err(_) => ("invalid", None),
    };
    document.insert("schema_version".into(), Value::String("0.9.0".into()));
    document.insert("adapter_sha256".into(), adapter_sha256);
    document.insert(
        "native_tool_version".into(),
        native_tool_version.map_or(Value::Null, Value::String),
    );
    document.insert(
        "rulepack_approval".into(),
        Value::String("unverified".into()),
    );
    document.insert(
        "workspace_binding".into(),
        Value::String(workspace_binding.into()),
    );
    document.insert(
        "workspace_id".into(),
        workspace_id.map_or(Value::Null, Value::String),
    );
    document.insert("operation".into(), Value::String("lint".into()));
    document.insert("run_id".into(), Value::String(run_id));
    document.insert("language".into(), Value::String("python".into()));
    document.insert("command_status".into(), Value::String("incomplete".into()));
    document.insert("exit_code".into(), Value::from(3));
    document.insert("tool_approval".into(), Value::String("unverified".into()));
    Ok(feedback)
}

/// 只给本轮已运行的 CodeGuard 可执行制品生成适配器身份，不能证明它已获策略批准。
pub(crate) fn current_adapter_sha256() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let bytes = read_bounded_regular_file(&path, 128 * 1024 * 1024).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}

fn annotate_rulepack(feedback: &mut Value, tool_version: Option<&str>) {
    let pack = bundled_ruff_rulepack().ok();
    let compatible = pack.as_ref().is_some_and(|pack| {
        tool_version.is_some_and(|version| pack.supports_tool_version(version))
    });
    for file in feedback["files"].as_array_mut().into_iter().flatten() {
        let complete = file["run_status"] == "findings";
        for finding in file["findings"].as_array_mut().into_iter().flatten() {
            let mapping = if complete && compatible {
                pack.as_ref().and_then(|pack| {
                    finding["rule_id"]
                        .as_str()
                        .and_then(|rule| pack.mapping(rule))
                })
            } else {
                None
            };
            let object = finding.as_object_mut().expect("诊断反馈是对象");
            if let (Some(pack), Some(mapping)) = (&pack, mapping) {
                object.insert(
                    "codeguard_rule_id".into(),
                    Value::String(mapping.codeguard_rule_id.clone()),
                );
                object.insert("rulepack_sha256".into(), Value::String(pack.sha256.clone()));
                object.insert(
                    "rulepack_status".into(),
                    Value::String("candidate_unapproved".into()),
                );
            } else {
                object.insert("codeguard_rule_id".into(), Value::Null);
                object.insert("rulepack_sha256".into(), Value::Null);
                object.insert(
                    "rulepack_status".into(),
                    Value::String(
                        if !complete {
                            "unverified_incomplete"
                        } else if pack.is_none() {
                            "rulepack_unavailable"
                        } else {
                            "unmapped_or_unvalidated_version"
                        }
                        .into(),
                    ),
                );
            }
        }
    }
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    if args.first().map(String::as_str) != Some("python") {
        return Err("当前 lint 仅接入 python 的局部原生 Ruff 检查".into());
    }
    let mut root = None;
    let mut json = false;
    let mut ruff_tool = None;
    let mut selected_paths = Vec::new();
    let mut timeout_ms = 0;
    let mut timeout_seen = false;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" || arg == "--ruff-tool" || arg == "--timeout" || arg == "--file" {
            index += 1;
            let value = args.get(index).ok_or_else(|| format!("{arg} 缺少值"))?;
            if arg == "--format" {
                json = parse_format(value)?;
            } else if arg == "--timeout" {
                if timeout_seen {
                    return Err("--timeout 重复".into());
                }
                timeout_ms = parse_check_timeout(value)?;
                timeout_seen = true;
            } else if arg == "--file" {
                selected_paths.push(value.clone());
            } else if ruff_tool.replace(PathBuf::from(value)).is_some() {
                return Err("--ruff-tool 重复".into());
            }
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    if ruff_tool.as_ref().is_some_and(|tool| !tool.is_absolute()) {
        return Err("--ruff-tool 必须是绝对路径".into());
    }
    let mut distinct = BTreeSet::new();
    let mut total_bytes = 0;
    for path in &selected_paths {
        if path.is_empty()
            || path.len() > 512
            || path.starts_with('/')
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err("--file 必须是有效的工作区相对源码路径".into());
        }
        if distinct.insert(path.clone()) {
            total_bytes += path.len();
        }
    }
    if distinct.len() > 8 || total_bytes > 2048 {
        return Err("--file 超出局部快检范围预算，请改用批量检查".into());
    }
    let selected_paths = distinct.into_iter().collect();
    let (timeout_ms, timeout_source) = select_check_timeout(timeout_seen.then_some(timeout_ms))?;
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
        ruff_tool,
        selected_paths,
        timeout_ms,
        timeout_source,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err(format!("不支持的格式：{value}")),
    }
}

fn prepare_tool(
    root: &Path,
    requested: Option<&Path>,
    deadline: Instant,
    run_id: &str,
    cancelled: &AtomicBool,
) -> ToolSetup {
    let unavailable = |reason| ToolSetup {
        path: None,
        sha256: [0; 32],
        version: String::new(),
        unavailable_reason: reason,
        scratch: None,
    };
    if Instant::now() >= deadline {
        return unavailable("request_deadline_exceeded");
    }
    let path = match crate::ruff_tool_selection::resolve_ruff_tool(root, requested) {
        Ok(Some(path)) => path,
        Ok(None) => return unavailable("ruff_tool_not_found"),
        Err(reason) => return unavailable(reason),
    };
    let Ok(content) = read_bounded_regular_file(&path, 128 * 1024 * 1024) else {
        return unavailable("ruff_tool_unreadable");
    };
    let sha256 = Sha256::digest(&content).into();
    let Ok(scratch) = create_scratch(run_id) else {
        return unavailable("evidence_storage_unavailable");
    };
    let version = ProcessSpec {
        executable: path.clone(),
        args: vec!["--version".into()],
        cwd: root.to_path_buf(),
        env: BTreeMap::new(),
        stdin: None,
        deadline,
        output_limit_bytes: 4096,
    };
    let outcome =
        match run_process_recorded(&version, cancelled, &scratch.0, "cli-ruff-version.log") {
            Ok(outcome) => outcome,
            Err(_) => {
                return unavailable(if codeguard_runtime::sigint_cancellation_requested() {
                    "request_cancelled"
                } else if Instant::now() >= deadline {
                    "request_deadline_exceeded"
                } else {
                    "ruff_version_unavailable"
                });
            }
        };
    let Ok(version) = std::str::from_utf8(&outcome.stdout) else {
        return unavailable("ruff_version_unavailable");
    };
    let version = version.strip_suffix('\n').unwrap_or("");
    if outcome.termination != Termination::Exited(0)
        || !outcome.stderr.is_empty()
        || !valid_ruff_version(version)
    {
        return unavailable(if outcome.termination == Termination::Cancelled {
            "request_cancelled"
        } else if Instant::now() >= deadline {
            "request_deadline_exceeded"
        } else {
            "ruff_version_unavailable"
        });
    }
    ToolSetup {
        path: Some(path),
        sha256,
        version: version.into(),
        unavailable_reason: "ruff_tool_not_found",
        scratch: Some(scratch),
    }
}

fn valid_ruff_version(version: &str) -> bool {
    let Some(number) = version.strip_prefix("ruff ") else {
        return false;
    };
    !number.is_empty()
        && number.len() < 80
        && number.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'(' | b')' | b' ')
        })
}

fn run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("lint-{}-{nanos}", std::process::id())
}

fn create_scratch(run_id: &str) -> std::io::Result<PrivateScratch> {
    let root = std::env::temp_dir().canonicalize()?;
    let path = root.join(format!("codeguard-{run_id}"));
    fs::create_dir(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    Ok(PrivateScratch(path))
}

fn print_human(feedback: &Value) {
    println!("CodeGuard Python lint：局部检查；完整质量门禁尚未评估");
    println!(
        "本轮范围：{}；目标：{}",
        feedback["scan_scope"].as_str().unwrap_or("unknown"),
        feedback["requested_paths"]
    );
    println!(
        "修复记录：{}",
        feedback["backlog_status"].as_str().unwrap_or("unknown")
    );
    if feedback["backlog_status"] == "synced_partial" {
        println!(
            "本次新增源码问题 {} 项、环境/配置任务 {} 项；下一步查看 .codeguard/tasks/ 并按任务复检。",
            feedback["backlog_sync"]["new_findings"]
                .as_u64()
                .unwrap_or(0),
            feedback["backlog_sync"]["new_blockers"]
                .as_u64()
                .unwrap_or(0)
        );
    }
    if feedback["repair_brief_status"] == "available" {
        let next = &feedback["next"];
        println!(
            "下一步：{}；原因 {}。",
            next["disposition"].as_str().unwrap_or("unknown"),
            next["reason"].as_str().unwrap_or("unknown")
        );
        if next["repair_brief"].is_object() {
            println!(
                "任务 {}：{}；范围 {}；复检 {}",
                next["repair_brief"]["task_id"],
                next["repair_brief"]["step"],
                next["repair_brief"]["scope"],
                next["repair_brief"]["recheck_argv"]
            );
        }
    } else {
        println!(
            "修复简报不可用：{}",
            feedback["repair_brief_reason"]
                .as_str()
                .unwrap_or("unknown")
        );
    }
    for checker in feedback["checker_configurations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        println!(
            "检查器 {}：配置 {}；位置 {}；建议 {}",
            quoted(&checker["checker_id"]),
            checker["configuration"].as_str().unwrap_or("unknown"),
            quoted(&checker["configuration_ref"]),
            quoted(&checker["next_action"]),
        );
    }
    for file in feedback["files"].as_array().into_iter().flatten() {
        println!(
            "文件 {}：{}{}",
            quoted(&file["path"]),
            file["run_status"].as_str().unwrap_or("incomplete"),
            file["reason"]
                .as_str()
                .map_or_else(String::new, |reason| format!("（{reason}）"))
        );
        if !file["rule_settings"].is_null() {
            println!(
                "  Ruff 全局启用的已映射规则：{}；逐文件忽略配置：{}；规则覆盖尚未证明",
                file["rule_settings"]["globally_enabled_mapped_rules"],
                file["rule_settings"]["per_file_ignores_present"]
            );
        }
        if !file["suppression_audit"].is_null() {
            println!(
                "  原生注释抑制观察：{} 处，规则 {}；不计入普通违规，仍需策略核验",
                file["suppression_audit"]["suppressed_diagnostic_count"],
                file["suppression_audit"]["suppressed_rule_ids"]
            );
        }
        for finding in file["findings"].as_array().into_iter().flatten() {
            let hint = &finding["repair_hint"];
            println!(
                "  {}:{}:{} {}：{}；{}",
                quoted(&finding["path"]),
                finding["line"],
                finding["column"],
                quoted(&finding["rule_id"]),
                finding["rule_summary"].as_str().unwrap_or("查看原生规则"),
                hint["step"].as_str().unwrap_or("先核对原生证据"),
            );
            if hint["status"] == "bounded_repair_candidate" {
                println!(
                    "    限定文件 {}；修复后用原生工具复检",
                    quoted(&hint["allowed_path"])
                );
            }
        }
    }
    if feedback["syntax_precheck"].is_object() {
        let precheck = &feedback["syntax_precheck"];
        println!(
            "内置 Python 语法候选初检：已检查 {} / {}，疑似位置 {}，状态 incomplete；候选涉及文件的原生 Ruff 未运行。",
            precheck["checked_files"],
            precheck["selected_files"],
            precheck["observations"].as_array().map_or(0, Vec::len)
        );
        for row in precheck["observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "疑似语法 {}:{}:{} {}；须由原生工具确认。",
                row["path"], row["start_line"], row["start_column"], row["kind"]
            );
        }
        println!(
            "下一步：{}",
            feedback["next_action"]
                .as_str()
                .unwrap_or("复查原生检查条件")
        );
        for row in precheck["structural_observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "候选结构 {}：{}；父节点 {}；原始零基字节位置 {}:{}；须由原生工具确认。",
                row["path"],
                row["rule_id"],
                row["parent_syntax_kind"],
                row["start_row"],
                row["start_column_byte"]
            );
        }
        if let Some(id) = feedback["setup"]["task_id"].as_str() {
            println!("原生确认任务：{id}");
        } else if let Some(reason) = feedback["task_persistence"]["reason"].as_str() {
            println!("确认任务未保存：{reason}");
        }
    }
    println!("本命令尚无批准策略与完整义务，交付判定：not_evaluated");
}

fn quoted(value: &Value) -> String {
    serde_json::to_string(value.as_str().unwrap_or("<missing>")).expect("字符串可编码")
}

#[cfg(test)]
mod rulepack_annotation_tests {
    use serde_json::json;

    use super::annotate_rulepack;

    #[test]
    fn unknown_rule_or_tool_version_keeps_native_diagnostic_without_mapping() {
        let mut feedback = json!({"files":[{"run_status":"findings","findings":[
            {"rule_id":"F401"},{"rule_id":"F999"}
        ]}]});
        annotate_rulepack(&mut feedback, Some("ruff 0.16.9"));
        for finding in feedback["files"][0]["findings"].as_array().unwrap() {
            assert!(finding["codeguard_rule_id"].is_null());
            assert!(finding["rulepack_sha256"].is_null());
            assert_eq!(
                finding["rulepack_status"],
                "unmapped_or_unvalidated_version"
            );
        }
        assert_eq!(feedback["files"][0]["findings"][0]["rule_id"], "F401");
        assert_eq!(feedback["files"][0]["findings"][1]["rule_id"], "F999");
    }
}
