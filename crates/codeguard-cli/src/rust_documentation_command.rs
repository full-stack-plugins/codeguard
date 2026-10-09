//! Rust 注释统一入口；保留两项原生观察，不授予详细契约或生产资格。
use crate::check_budget::{budget_record, resolve_project_default, select_check_timeout};
use crate::discovery::discover;
use codeguard_runtime::{NativeObservation, SourceSnapshot};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// 解析并执行统一注释检查；参数为语种后的 argv，返回未完成或取消退出码。
pub(crate) fn run(args: &[String]) -> ExitCode {
    let arguments = match crate::rust_comments_command::parse_args(args) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let (timeout, source) = match select_check_timeout(arguments.timeout)
        .and_then(|(value, source)| resolve_project_default(&arguments.root, value, source))
    {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let cancelled = AtomicBool::new(false);
    let tool = crate::cargo_tool_selection::resolve_cargo_tool(arguments.tool.as_deref());
    let mut rustdoc = crate::rust_comments_command::empty_report(timeout, source);
    let mut clippy = Value::Null;
    let mut input_stable = false;
    let mut next = Value::Null;
    if let Some(root) = arguments
        .root
        .canonicalize()
        .ok()
        .filter(|root| root.is_dir())
    {
        let discovery = codeguard_adapters::legacy_registry()
            .ok()
            .map(|registry| discover(&root, &registry, &NativeObservation));
        let files = discovery
            .as_ref()
            .filter(|d| d.observation_complete)
            .and_then(|d| d.languages.get("rust"))
            .map(|language| language.source_files.clone());
        let snapshot = files.as_ref().and_then(|files| {
            let mut paths: Vec<_> = files.iter().map(PathBuf::from).collect();
            paths.extend([PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")]);
            SourceSnapshot::capture(&root, paths, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024).ok()
        });
        // 两次进程调用共用截止时间；后一项不能获得新的完整超时预算。
        rustdoc = crate::rust_comments_command::observe_for_verification(
            &root,
            tool.as_deref().or(arguments.tool.as_deref()),
            timeout,
            source,
            deadline,
            false,
            &cancelled,
        );
        crate::rust_comments_command::persist_and_sync(&root, &mut rustdoc);
        if let Some(files) = files.as_ref() {
            clippy = crate::rust_lint_scan::observe_cargo_clippy(
                &root,
                files,
                tool.as_deref().or(arguments.tool.as_deref()),
                deadline,
                &cancelled,
            );
            if clippy["workspace_binding"] == "bound" {
                crate::rust_lint_workbench::persist_and_sync(&root, &mut clippy);
            }
        }
        input_stable = snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.verify_source_unchanged().unwrap_or(false));
        // 零诊断仍保留历史开放文档任务，由原工具复检确认；无关规范任务不入选。
        next = crate::next_command::read_rust_documentation_brief(&root).unwrap_or(Value::Null);
    }
    let tool_identity_consistent =
        rustdoc["tool_sha256"].is_string() && rustdoc["tool_sha256"] == clippy["tool_sha256"];
    let complete = input_stable
        && tool_identity_consistent
        && rustdoc["local_scan_complete"] == true
        && clippy["local_scan_complete"] == true;
    let interrupted = rustdoc["reason"] == "request_cancelled"
        || clippy["reason"] == "request_cancelled"
        || codeguard_runtime::sigint_cancellation_requested();
    let exit = if interrupted { 130 } else { 3 };
    let documentation_findings: Vec<_> = clippy["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| documentation_rule(finding))
        .cloned()
        .collect();
    let feedback = json!({"schema_version":"0.1.0","report_type":"rust_comments_feedback","operation":"comments","language":"rust",
        "command_status":if interrupted{"cancelled"}else{"incomplete"},"exit_code":exit,
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "execution_budget":budget_record(timeout,source),"local_scan_complete":complete,"input_stable":input_stable,
        "tool_identity_consistent":tool_identity_consistent,"native_results":{"rustdoc":rustdoc,"clippy":clippy},
        "documentation_findings":documentation_findings,"next":next,
        "detailed_contract_qualification":"not_granted",
        "next_actions":["分别核对 rustdoc 探针规则和原项目 Clippy 文档规则；不会隐式启用 pedantic。",
            "按稳定任务使用 task verify 原工具复检；裸章节标题或局部零诊断不证明详细文档合规。"]});
    if arguments.json {
        println!("{feedback}");
    } else {
        println!(
            "Rust 注释检查：双原生局部完成 {}；详细契约资格未授予。",
            complete
        );
        for checker in ["rustdoc", "clippy"] {
            println!(
                "{}：完成 {}；原因 {}",
                checker,
                feedback["native_results"][checker]["local_scan_complete"],
                feedback["native_results"][checker]["reason"]
            );
        }
        for finding in feedback["native_results"]["rustdoc"]["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                feedback["documentation_findings"]
                    .as_array()
                    .into_iter()
                    .flatten(),
            )
        {
            println!(
                "原生规则 {}：{}:{}；任务 {}，按原工具复检。",
                finding["rule_id"], finding["path"], finding["line"], finding["finding_id"]
            );
        }
        println!("下一步：{}", feedback["next"]);
    }
    ExitCode::from(exit)
}

fn documentation_rule(finding: &Value) -> bool {
    matches!(
        finding["rule_id"].as_str(),
        Some(
            "clippy::missing_errors_doc"
                | "clippy::missing_panics_doc"
                | "clippy::missing_safety_doc"
        )
    )
}
