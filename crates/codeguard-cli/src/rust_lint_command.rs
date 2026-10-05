//! Rust独立lint复用Clippy原生观察和稳定任务，不执行其它检查族。
use crate::{
    check_budget::{budget_record, resolve_project_default, select_check_timeout},
    discovery::discover,
    rust_lint_arguments::RustLintArguments,
};
use serde_json::{Value, json};
use std::{
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
/// 执行Rust局部lint；参数为该语种后argv，返回操作退出码和可读/JSON反馈。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match RustLintArguments::parse(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (timeout, source) = match select_check_timeout(arguments.timeout)
        .and_then(|(v, s)| resolve_project_default(&arguments.root, v, s))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let selected = crate::cargo_tool_selection::resolve_cargo_tool(arguments.tool.as_deref());
    let selection = json!({"source":if arguments.tool.is_some(){"explicit"}else if selected.is_some(){"path"}else{"not_found"},"executable":selected});
    let root = arguments.root.canonicalize().ok().filter(|p| p.is_dir());
    let mut native = Value::Null;
    let mut syntax = crate::rust_lint_fallback::unavailable("project_root_unavailable");
    let mut tasks = Value::Null;
    let mut next = Value::Null;
    if let Some(root) = root.as_deref() {
        let sources = codeguard_adapters::legacy_registry()
            .ok()
            .map(|registry| discover(root, &registry, &codeguard_runtime::NativeObservation));
        let files = sources
            .as_ref()
            .and_then(|d| d.languages.get("rust"))
            .map(|l| l.source_files.clone())
            .unwrap_or_default();
        if sources.as_ref().is_some_and(|d| d.observation_complete) && !files.is_empty() {
            native = crate::rust_lint_scan::observe_cargo_clippy(
                root,
                &files,
                selected.as_deref(),
                deadline,
                &AtomicBool::new(false),
            );
            // 保存原有原生协议，再同步任务；公开封装不能冒充原生扫描记录。
            crate::rust_lint_workbench::persist_and_sync(root, &mut native);
            if selected.is_none() && arguments.tool.is_none() {
                syntax = crate::rust_lint_fallback::observe(root, deadline);
                tasks = crate::syntax_confirmation::persist(root, &syntax, deadline);
            } else {
                syntax = crate::rust_lint_fallback::unavailable("native_tool_selected");
            }
        } else {
            syntax = crate::rust_lint_fallback::unavailable("rust_source_scope_unavailable");
        }
        next = crate::next_command::read_local_brief_for_checker(root, "rust.cargo_clippy")
            .unwrap_or(Value::Null);
        if next.is_null() {
            next = crate::next_command::read_local_brief(root).unwrap_or(Value::Null);
        }
    }
    let cancelled = native["reason"] == "request_cancelled"
        || codeguard_runtime::sigint_cancellation_requested();
    let result = if selected.is_some() {
        "not_run"
    } else {
        crate::rust_lint_fallback::preliminary_result(&syntax)
    };
    let requirement = if selected.is_some() && native["local_scan_complete"] == true {
        "available"
    } else if result == "no_candidates_observed" {
        "recommended"
    } else {
        "required"
    };
    let exit = if cancelled { 130 } else { 3 };
    let feedback = json!({"schema_version":"0.1.0","report_type":"rust_lint_feedback","operation":"lint","language":"rust","command_status":if cancelled{"cancelled"}else{"incomplete"},"exit_code":exit,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","execution_budget":budget_record(timeout,source),"tool_selection":selection,"native_report":native,"syntax_candidates":syntax,"syntax_tasks":tasks,"next":next,"preliminary_result":result,"native_tool_requirement":requirement});
    if arguments.json {
        println!("{feedback}");
    } else {
        println!(
            "Rust lint：原生局部完成 {}；原因 {}；WASM初检 {}；原生工具准备 {}",
            feedback["native_report"]["local_scan_complete"],
            feedback["native_report"]["reason"],
            result,
            requirement
        );
        for finding in feedback["native_report"]["findings"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "规则 {}：{}:{}；按原规则修复后使用 task verify 原工具复检",
                finding["rule_id"], finding["path"], finding["line"]
            );
        }
        println!(
            "任务同步 {}；下一步 {}；完整覆盖与交付未评估，零诊断不关闭任务。",
            feedback["native_report"]["backlog_status"], feedback["next"]
        );
    }
    ExitCode::from(exit)
}
