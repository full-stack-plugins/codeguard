//! 宿主事件的局部执行入口；当前只把项目发现与 Python 编辑快检接到 Rust 检查链。

use crate::check_budget::parse_check_timeout;
use crate::discovery::discover;
use crate::hook_plan_command::parse_request;
use crate::python_lint_command::{
    annotate_conversation_budget, scan_selected_report_with_deadline,
};
use codeguard_adapters::legacy_registry;
use codeguard_core::{HookTriggerAction, plan_hook_trigger};
use codeguard_runtime::NativeObservation;
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const MAX_FAST_TIMEOUT_MS: u64 = 120_000;

struct Arguments {
    root: PathBuf,
    timeout_ms: u64,
    ruff_tool: Option<PathBuf>,
}

/// 从 `args` 与 stdin 读取宿主事件，按纯路由执行已接入的局部动作。
/// 返回 CLI 退出码；所有输出保持交付未评估，不能替代宿主协议映射。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match parse_args(args) {
        Ok(arguments) => arguments,
        Err(reason) => {
            eprintln!("hook execute 参数无效：{reason}");
            return ExitCode::from(2);
        }
    };
    let mut raw = Vec::new();
    match std::io::stdin()
        .lock()
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut raw)
    {
        Ok(_) if raw.is_empty() || raw.len() as u64 > MAX_REQUEST_BYTES => {
            eprintln!("hook 请求为空或超过 64 KiB");
            return ExitCode::from(2);
        }
        Ok(_) => {}
        Err(_) => {
            eprintln!("读取 hook 请求失败");
            return ExitCode::from(4);
        }
    }
    let input = match parse_request(&raw) {
        Ok(input) => input,
        Err(reason) => {
            eprintln!("无效 hook 请求：{reason}");
            return ExitCode::from(2);
        }
    };
    let plan = match plan_hook_trigger(&input) {
        Ok(plan) => plan,
        Err(reason) => {
            eprintln!("无效 hook 事件：{reason}");
            return ExitCode::from(2);
        }
    };
    let root = arguments
        .root
        .canonicalize()
        .ok()
        .filter(|root| root.is_dir());
    let (execution, reason, feedback, exit_code) = match (plan.action, root.as_deref()) {
        (_, None) => ("not_run", Some("workspace_unavailable"), Value::Null, 3),
        (HookTriggerAction::DiscoverProject, Some(root)) => match discovery_summary(root) {
            Ok(feedback) => ("read_only_discovery", None, feedback, 3),
            Err(_) => ("not_run", Some("registry_unavailable"), Value::Null, 4),
        },
        (HookTriggerAction::FastFileCheck, Some(root))
            if plan.target_paths.iter().all(|path| path.ends_with(".py")) =>
        {
            let deadline = Instant::now() + Duration::from_millis(arguments.timeout_ms);
            match scan_selected_report_with_deadline(
                root,
                arguments.ruff_tool.as_deref(),
                &plan.target_paths,
                deadline,
                &AtomicBool::new(false),
            ) {
                Ok(mut feedback) => {
                    annotate_conversation_budget(&mut feedback, arguments.timeout_ms, "cli");
                    feedback["schema_version"] = Value::String("0.13.0".into());
                    let cancelled = codeguard_runtime::sigint_cancellation_requested()
                        || feedback["incomplete_reasons"]
                            .as_array()
                            .is_some_and(|reasons| {
                                reasons.iter().any(|reason| reason == "request_cancelled")
                            });
                    if cancelled {
                        feedback["command_status"] = Value::String("cancelled".into());
                        feedback["exit_code"] = Value::from(130);
                    }
                    (
                        "local_observation",
                        cancelled.then_some("request_cancelled"),
                        feedback,
                        if cancelled { 130 } else { 3 },
                    )
                }
                Err(_) => ("not_run", Some("adapter_unavailable"), Value::Null, 4),
            }
        }
        (HookTriggerAction::FastFileCheck, Some(_)) => {
            ("not_run", Some("fast_scope_not_wired"), Value::Null, 3)
        }
        (HookTriggerAction::NoCheck, Some(_)) => ("not_run", Some("write_failed"), Value::Null, 3),
        (HookTriggerAction::ResolveChangedScope, Some(_)) => {
            ("not_run", Some("scope_resolution_required"), Value::Null, 3)
        }
        (
            HookTriggerAction::CommitGate
            | HookTriggerAction::PushGate
            | HookTriggerAction::FullProjectCheck,
            Some(_),
        ) => ("not_run", Some("delivery_gate_not_wired"), Value::Null, 3),
        (_, Some(_)) => ("not_run", Some("action_not_wired"), Value::Null, 3),
    };
    println!(
        "{}",
        json!({
            "schema_version":"0.1.0", "report_type":"hook_execution_feedback",
            "plan":plan, "execution":execution, "reason":reason,
            "local_feedback":feedback, "delivery_decision":"not_evaluated",
            "host_blocking_verified":false, "soft_result_reused":false
        })
    );
    ExitCode::from(exit_code)
}

fn discovery_summary(root: &Path) -> Result<Value, &'static str> {
    let registry = legacy_registry().map_err(|_| "registry_invalid")?;
    let discovered = discover(root, &registry, &NativeObservation);
    let configurations: Vec<Value> = discovered
        .checker_configurations
        .iter()
        .take(32)
        .map(|checker| {
            json!({"checker_id":checker.checker_id,
                "build_root":checker.build_root,
                "configuration":checker.configuration})
        })
        .collect();
    Ok(json!({
        "schema_version":"0.1.0", "report_type":"hook_discovery_summary",
        "languages": discovered.languages.keys().collect::<Vec<_>>(),
        "checker_configurations":configurations,
        "checker_count":discovered.checker_configurations.len(),
        "checker_list_truncated":discovered.checker_configurations.len()>32,
        "observation_complete":discovered.observation_complete,
        "unknown_condition_count":discovered.unknown_conditions.len(),
        "source_check":"not_run", "delivery_decision":"not_evaluated"
    }))
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let Some(root) = args.first().filter(|value| !value.starts_with('-')) else {
        return Err("需要项目路径".into());
    };
    let mut timeout_ms = None;
    let mut ruff_tool = None;
    let mut format_seen = false;
    let mut index = 1;
    while index < args.len() {
        let raw = &args[index];
        let (key, value) = if let Some((key, value)) = raw.split_once('=') {
            (key, value.to_owned())
        } else {
            index += 1;
            (raw.as_str(), args.get(index).ok_or("选项缺少值")?.clone())
        };
        match key {
            "--format" if value == "json" && !format_seen => format_seen = true,
            "--timeout" if timeout_ms.is_none() => {
                timeout_ms = Some(parse_check_timeout(&value)?);
            }
            "--ruff-tool" if ruff_tool.is_none() => {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("--ruff-tool 必须为绝对路径".into());
                }
                ruff_tool = Some(path);
            }
            _ => return Err(format!("不支持或重复的参数：{key}")),
        }
        index += 1;
    }
    if !format_seen {
        return Err("需要 --format=json".into());
    }
    let timeout_ms = timeout_ms.ok_or("需要 --timeout")?;
    if timeout_ms > MAX_FAST_TIMEOUT_MS {
        return Err("局部事件预算超过 120s".into());
    }
    Ok(Arguments {
        root: PathBuf::from(root),
        timeout_ms,
        ruff_tool,
    })
}
