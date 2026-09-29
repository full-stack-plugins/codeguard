//! 宿主事件的局部执行入口；接入只读发现、Stop 指引、任务复检、编辑快检和暂存面观察。

use crate::check_budget::parse_check_timeout;
use crate::discovery::discover;
use crate::git_index_safety::observe_index_safety_with_deadline;
use crate::hook_plan_command::parse_request;
use crate::next_command::{read_local_brief, read_task_brief};
use crate::python_lint_command::{
    annotate_conversation_budget, scan_selected_report_with_deadline,
};
use codeguard_adapters::legacy_registry;
use codeguard_core::{HookTriggerAction, HookTriggerInput, plan_hook_trigger};
use codeguard_runtime::{NativeObservation, ProcessSpec, Termination, run_process};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const MAX_FAST_TIMEOUT_MS: u64 = 120_000;
const MAX_GUIDANCE_RECORDS: usize = 64;
const MAX_GUIDANCE_REPORT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_VERIFY_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_VERIFY_OPTION_BYTES: usize = 16 * 1024;
const MAX_VERIFY_EVENTS: usize = 128;
const MAX_VERIFY_EVENT_BYTES: u64 = 1024 * 1024;

struct Arguments {
    root: PathBuf,
    timeout_ms: u64,
    ruff_tool: Option<PathBuf>,
    git_tool: Option<PathBuf>,
    verify_options: BTreeMap<String, String>,
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
    let (report, exit_code) = match execute_parsed(&arguments, &input) {
        Ok(result) => result,
        Err(reason) => {
            eprintln!("无效 hook 事件：{reason}");
            return ExitCode::from(2);
        }
    };
    println!("{report}");
    ExitCode::from(exit_code)
}

/// 使用同一事件执行器处理已由宿主适配器规范化的输入。
/// 参数为原 `hook execute` 选项和受限事件；返回原始 Rust 报告与其 CLI 退出码。
pub(crate) fn execute_host_input(
    args: &[String],
    input: &HookTriggerInput,
) -> Result<(Value, u8), String> {
    let arguments = parse_args(args)?;
    execute_parsed(&arguments, input).map_err(str::to_owned)
}

fn execute_parsed(
    arguments: &Arguments,
    input: &HookTriggerInput,
) -> Result<(Value, u8), &'static str> {
    let plan = plan_hook_trigger(input)?;
    if plan.action != HookTriggerAction::VerifyTask && !arguments.verify_options.is_empty() {
        return Err("任务复检参数仅用于 repair_ready 事件");
    }
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
        (HookTriggerAction::ShowSummary, Some(root)) => match guidance_summary(root) {
            Ok(feedback) => ("read_only_guidance", None, feedback, 3),
            Err("guidance_scope_exceeded") => {
                ("not_run", Some("guidance_scope_exceeded"), Value::Null, 3)
            }
            Err(_) => ("not_run", Some("guidance_unavailable"), Value::Null, 3),
        },
        (HookTriggerAction::VerifyTask, Some(root)) => {
            let task_id = plan.task_id.as_deref().expect("核心已校验任务 ID");
            match task_verification_summary(root, task_id, arguments) {
                Ok(feedback) => ("task_verification", None, feedback, 3),
                Err((reason, exit_code)) => ("not_run", Some(reason), Value::Null, exit_code),
            }
        }
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
        (HookTriggerAction::CommitGate, Some(root)) if arguments.git_tool.is_some() => {
            let tool = arguments.git_tool.as_deref().expect("guarded Git tool");
            let alternate_index = env::var_os("GIT_INDEX_FILE").map(PathBuf::from);
            let alternate_index = alternate_index.map(|index| {
                if index.is_absolute() {
                    index
                } else {
                    root.join(index)
                }
            });
            let deadline = Instant::now() + Duration::from_millis(arguments.timeout_ms.min(15_000));
            match observe_index_safety_with_deadline(
                root,
                tool,
                alternate_index.as_deref(),
                deadline,
            ) {
                Ok(observation) => {
                    let violations = observation
                        .violations
                        .iter()
                        .filter(|item| item.path.len() <= 512)
                        .take(32)
                        .map(|item| json!({"path":item.path,"rule_id":item.rule_id}))
                        .collect::<Vec<_>>();
                    let truncated = violations.len() != observation.violations.len();
                    let feedback = json!({
                        "schema_version":"0.1.0", "report_type":"hook_git_index_summary",
                        "index_observation":"complete", "index_listing_sha256":observation.listing_sha256,
                        "staged_entry_count":observation.entries.len(), "object_status":if observation.objects_verified {"verified"} else {"unresolved"},
                        "violation_count":observation.violations.len(), "violations_truncated":truncated,
                        "violations":violations, "source_check":"not_run",
                        "delivery_decision":"not_evaluated"
                    });
                    ("local_observation", None, feedback, 3)
                }
                Err(_) if codeguard_runtime::sigint_cancellation_requested() => {
                    ("not_run", Some("request_cancelled"), Value::Null, 130)
                }
                Err(_) if Instant::now() >= deadline => {
                    ("not_run", Some("request_deadline_exceeded"), Value::Null, 3)
                }
                Err(_) => (
                    "not_run",
                    Some("git_index_observation_failed"),
                    Value::Null,
                    3,
                ),
            }
        }
        (
            HookTriggerAction::CommitGate
            | HookTriggerAction::PushGate
            | HookTriggerAction::FullProjectCheck,
            Some(_),
        ) => ("not_run", Some("delivery_gate_not_wired"), Value::Null, 3),
        (_, Some(_)) => ("not_run", Some("action_not_wired"), Value::Null, 3),
    };
    Ok((
        json!({
            "schema_version":"0.4.0", "report_type":"hook_execution_feedback",
            "plan":plan, "execution":execution, "reason":reason,
            "local_feedback":feedback, "delivery_decision":"not_evaluated",
            "host_blocking_verified":false, "soft_result_reused":false
        }),
        exit_code,
    ))
}

fn task_verification_summary(
    root: &Path,
    task_id: &str,
    arguments: &Arguments,
) -> Result<Value, (&'static str, u8)> {
    let deadline = Instant::now() + Duration::from_millis(arguments.timeout_ms);
    check_verification_history_budget(root, task_id)?;
    let brief = read_task_brief(root, task_id).map_err(|_| ("task_unavailable", 3))?;
    let checker_id = brief["checker_id"]
        .as_str()
        .ok_or(("task_unavailable", 3))?;
    if (arguments.ruff_tool.is_some()
        && !matches!(checker_id, "python.ruff" | "python.ruff.doctor"))
        || arguments
            .verify_options
            .keys()
            .any(|key| !verify_option_matches_checker(key, checker_id))
    {
        return Err(("verification_arguments_invalid", 3));
    }
    let executable = env::current_exe().map_err(|_| ("verification_process_failed", 4))?;
    let remaining_ms = deadline
        .saturating_duration_since(Instant::now())
        .as_millis();
    if remaining_ms == 0 {
        return Err(("request_deadline_exceeded", 3));
    }
    let mut args = vec![
        OsString::from("task"),
        OsString::from("verify"),
        OsString::from(task_id),
        root.as_os_str().to_owned(),
        OsString::from("--timeout"),
        OsString::from(format!("{remaining_ms}ms")),
        OsString::from("--format=json"),
    ];
    if let Some(ruff_tool) = &arguments.ruff_tool {
        args.extend([
            OsString::from("--ruff-tool"),
            ruff_tool.as_os_str().to_owned(),
        ]);
    }
    for (key, value) in &arguments.verify_options {
        args.extend([OsString::from(key), OsString::from(value)]);
    }
    let mut child_env = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "TMPDIR",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "JAVA_HOME",
    ] {
        if let Some(value) = env::var_os(key) {
            child_env.insert(OsString::from(key), value);
        }
    }
    let outcome = run_process(
        &ProcessSpec {
            executable,
            args,
            cwd: root.to_path_buf(),
            env: child_env,
            stdin: None,
            deadline,
            output_limit_bytes: MAX_VERIFY_OUTPUT_BYTES,
        },
        &AtomicBool::new(false),
    );
    match outcome.termination {
        Termination::Exited(3) => {}
        Termination::Exited(2) => return Err(("verification_arguments_invalid", 3)),
        Termination::Exited(130) | Termination::Cancelled => {
            return Err(("request_cancelled", 130));
        }
        Termination::TimedOut | Termination::DeadlineBeforeStart => {
            return Err(("request_deadline_exceeded", 3));
        }
        Termination::OutputLimit => return Err(("verification_output_exceeded", 3)),
        _ => return Err(("verification_process_failed", 4)),
    }
    let report: Value =
        serde_json::from_slice(&outcome.stdout).map_err(|_| ("verification_report_invalid", 4))?;
    if report["report_type"] != "task_verification_preview"
        || report["operation"] != "task_verify"
        || report["task_id"] != task_id
        || report["exit_code"] != 3
        || report["authority"] != "local_unverified"
        || report["delivery_decision"] != "not_evaluated"
        || !report["event_persisted"].is_boolean()
    {
        return Err(("verification_report_invalid", 4));
    }
    let observation = report["observation"]
        .as_str()
        .filter(|value| value.len() <= 64)
        .ok_or(("verification_report_invalid", 4))?;
    let reason = report["reason"].as_str().filter(|value| value.len() <= 128);
    if !report["reason"].is_null() && reason.is_none() {
        return Err(("verification_report_invalid", 4));
    }
    Ok(json!({
        "schema_version":"0.1.0", "report_type":"hook_task_verification_summary",
        "task_id":task_id, "checker_id":checker_id,
        "observation":observation, "event_persisted":report["event_persisted"],
        "reason":reason, "scan_report_available":report["native_scan"].is_object(),
        "authority":"local_unverified", "delivery_decision":"not_evaluated"
    }))
}

fn check_verification_history_budget(root: &Path, task_id: &str) -> Result<(), (&'static str, u8)> {
    let events = root
        .join(".codeguard/findings")
        .join(task_id)
        .join("events");
    if !events.exists() {
        return Ok(());
    }
    if !fs::symlink_metadata(&events).is_ok_and(|metadata| metadata.file_type().is_dir()) {
        return Err(("task_unavailable", 3));
    }
    let mut total_bytes = 0_u64;
    for (index, entry) in fs::read_dir(events)
        .map_err(|_| ("task_unavailable", 3))?
        .enumerate()
    {
        if index >= MAX_VERIFY_EVENTS {
            return Err(("verification_scope_exceeded", 3));
        }
        let entry = entry.map_err(|_| ("task_unavailable", 3))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| ("task_unavailable", 3))?;
        if !metadata.file_type().is_file() {
            return Err(("task_unavailable", 3));
        }
        total_bytes = total_bytes.saturating_add(metadata.len());
        if total_bytes > MAX_VERIFY_EVENT_BYTES {
            return Err(("verification_scope_exceeded", 3));
        }
    }
    Ok(())
}

fn verify_option_matches_checker(key: &str, checker_id: &str) -> bool {
    if matches!(key, "--owner" | "--lease-token") {
        return true;
    }
    match checker_id {
        "node.eslint" | "node.eslint.preparation" => matches!(
            key,
            "--node-tool" | "--eslint-entry" | "--eslint-version" | "--config" | "--cwd"
        ),
        "node.npm.audit" => matches!(
            key,
            "--node-tool"
                | "--npm-entry"
                | "--npm-version"
                | "--userconfig"
                | "--globalconfig"
                | "--registry"
        ),
        "python.pip_audit" => matches!(key, "--pip-audit-tool" | "--pip-audit-version"),
        "go.vet" => key == "--go-tool",
        "rust.cargo_clippy" | "rust.cargo_check" | "rust.cargo_rustdoc" => key == "--cargo-tool",
        "rust.cargo_audit" => matches!(key, "--cargo-audit-tool" | "--rustsec-db"),
        "java.checkstyle" | "java.checkstyle.preparation" => {
            matches!(key, "--java-tool" | "--checkstyle-jar" | "--config")
        }
        "java.maven.p3c" => matches!(
            key,
            "--maven-tool" | "--java-home" | "--maven-repo" | "--repo-sha256"
        ),
        "java.maven.dependency_check" => matches!(
            key,
            "--maven-tool"
                | "--java-home"
                | "--maven-repo"
                | "--repo-sha256"
                | "--cve-data-dir"
                | "--cve-data-sha256"
        ),
        "python.ruff" | "python.ruff.doctor" => false,
        _ => false,
    }
}

fn guidance_summary(root: &Path) -> Result<Value, &'static str> {
    let workspace = root.join(".codeguard");
    if workspace.exists() {
        if !fs::symlink_metadata(&workspace).is_ok_and(|metadata| metadata.file_type().is_dir()) {
            return Err("guidance_unavailable");
        }
        for name in ["findings", "reports"] {
            let directory = workspace.join(name);
            if !directory.exists() {
                continue;
            }
            if !fs::symlink_metadata(&directory).is_ok_and(|metadata| metadata.file_type().is_dir())
            {
                return Err("guidance_unavailable");
            }
            let mut total_bytes = 0_u64;
            for (index, entry) in fs::read_dir(&directory)
                .map_err(|_| "guidance_unavailable")?
                .enumerate()
            {
                if index >= MAX_GUIDANCE_RECORDS {
                    return Err("guidance_scope_exceeded");
                }
                let entry = entry.map_err(|_| "guidance_unavailable")?;
                if name == "reports" {
                    let metadata =
                        fs::symlink_metadata(entry.path()).map_err(|_| "guidance_unavailable")?;
                    if !metadata.file_type().is_file() {
                        return Err("guidance_unavailable");
                    }
                    total_bytes = total_bytes.saturating_add(metadata.len());
                    if total_bytes > MAX_GUIDANCE_REPORT_BYTES {
                        return Err("guidance_scope_exceeded");
                    }
                }
            }
        }
    }
    let view = read_local_brief(root)?;
    let brief = &view["repair_brief"];
    let safe_string = |key: &str, limit: usize| {
        brief[key]
            .as_str()
            .filter(|value| value.len() <= limit)
            .map(str::to_owned)
    };
    let next_actions = if brief.is_object() {
        json!([["codeguard", "next", ".", "--format=json"]])
    } else {
        view["next_actions"].clone()
    };
    Ok(json!({
        "schema_version":"0.1.0", "report_type":"hook_next_guidance",
        "disposition":view["disposition"], "reason":view["reason"],
        "task_id":safe_string("task_id", 96),
        "checker_id":safe_string("checker_id", 96),
        "step":safe_string("step", 512),
        "next_actions":next_actions,
        "source_check":"not_run", "authority":"local_unverified",
        "delivery_decision":"not_evaluated"
    }))
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
    let mut git_tool = None;
    let mut verify_options = BTreeMap::new();
    let mut verify_option_bytes = 0_usize;
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
            "--git-tool" if git_tool.is_none() => {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("--git-tool 必须为绝对路径".into());
                }
                git_tool = Some(path);
            }
            "--cargo-tool"
            | "--cargo-audit-tool"
            | "--rustsec-db"
            | "--pip-audit-tool"
            | "--pip-audit-version"
            | "--go-tool"
            | "--maven-tool"
            | "--java-home"
            | "--java-tool"
            | "--checkstyle-jar"
            | "--config"
            | "--maven-repo"
            | "--repo-sha256"
            | "--cve-data-dir"
            | "--cve-data-sha256"
            | "--node-tool"
            | "--npm-entry"
            | "--npm-version"
            | "--userconfig"
            | "--globalconfig"
            | "--registry"
            | "--eslint-entry"
            | "--eslint-version"
            | "--cwd"
            | "--owner"
            | "--lease-token"
                if !verify_options.contains_key(key) =>
            {
                if matches!(
                    key,
                    "--cargo-tool"
                        | "--cargo-audit-tool"
                        | "--rustsec-db"
                        | "--pip-audit-tool"
                        | "--go-tool"
                        | "--maven-tool"
                        | "--java-home"
                        | "--java-tool"
                        | "--checkstyle-jar"
                        | "--config"
                        | "--maven-repo"
                        | "--cve-data-dir"
                        | "--node-tool"
                        | "--npm-entry"
                        | "--userconfig"
                        | "--globalconfig"
                        | "--eslint-entry"
                        | "--cwd"
                ) && !Path::new(&value).is_absolute()
                {
                    return Err(format!("{key} 必须为绝对路径"));
                }
                verify_option_bytes = verify_option_bytes.saturating_add(key.len() + value.len());
                if value.is_empty() || verify_option_bytes > MAX_VERIFY_OPTION_BYTES {
                    return Err("任务复检参数为空或超预算".into());
                }
                verify_options.insert(key.to_owned(), value);
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
        git_tool,
        verify_options,
    })
}
