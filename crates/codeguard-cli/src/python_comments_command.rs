//! Python 独立注释入口：复用原 Ruff 扫描，保持文档任务与开发规范任务的分类边界。
use crate::check_budget::{budget_record, resolve_project_default, select_check_timeout};
use crate::python_comments_arguments::PythonCommentsArguments;
use serde_json::{Value, json};
use std::{
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// 执行项目内已配置 Ruff 的注释观察；参数为语种后 argv，返回局部未完成或取消退出码。
pub fn run(args: &[String]) -> ExitCode {
    let arguments = match PythonCommentsArguments::parse(args) {
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
    let mut native = Value::Null;
    let mut next = Value::Null;
    let mut reason = "project_root_unavailable";
    if let Some(root) = arguments
        .root
        .canonicalize()
        .ok()
        .filter(|root| root.is_dir())
    {
        match crate::python_lint_command::scan_and_sync_report_with_deadline(
            &root,
            arguments.tool.as_deref(),
            deadline,
            &AtomicBool::new(false),
        ) {
            Ok(mut report) => {
                // 工作台已经保存原 0.9 扫描；这里仅引用已有 0.12 对话协议，不改写原始事实。
                crate::python_lint_command::annotate_conversation_budget(
                    &mut report,
                    timeout,
                    source,
                );
                native = report;
                reason = "native_documentation_coverage_unverified";
                if native["workspace_binding"] == "bound" {
                    next = crate::next_command::read_python_documentation_brief(&root)
                        .unwrap_or(Value::Null);
                }
            }
            Err(_) => reason = "language_registry_invalid",
        }
    }
    let interrupted = codeguard_runtime::sigint_cancellation_requested()
        || native["incomplete_reasons"]
            .as_array()
            .is_some_and(|reasons| reasons.iter().any(|reason| reason == "request_cancelled"));
    if interrupted && native.is_object() {
        native["command_status"] = json!("cancelled");
        native["exit_code"] = json!(130);
    }
    let findings: Vec<_> = native["files"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|file| file["findings"].as_array().into_iter().flatten())
        .filter(|finding| {
            finding["rule_id"]
                .as_str()
                .is_some_and(codeguard_adapters::is_ruff_documentation_rule)
        })
        .cloned()
        .collect();
    let complete = !interrupted && native["local_scan_complete"] == true;
    let exit = if interrupted { 130 } else { 3 };
    let report = json!({"schema_version":"0.1.0","report_type":"python_comments_feedback","operation":"comments","language":"python",
        "command_status":if interrupted{"cancelled"}else{"incomplete"},"exit_code":exit,"authority":"local_unverified","coverage_proven":false,
        "delivery_decision":"not_evaluated","local_scan_complete":complete,"reason":reason,"execution_budget":budget_record(timeout,source),
        "native_report":native,"documentation_findings":findings,"next":next,"documentation_rule_coverage":"unverified","detailed_contract_qualification":"not_granted",
        "next_actions":["核对原项目 Ruff 文档规则和配置；不会隐式启用 preview 或改写规则选择。",
            "按稳定任务使用 task verify 原工具复检；零诊断不证明文档规则完整启用、详细内容准确或任务已关闭。"]});
    if arguments.json {
        println!("{report}");
    } else {
        println!(
            "Python 注释检查：原生局部完成 {}；文档规则完整覆盖未验证。",
            complete
        );
        for finding in report["documentation_findings"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "原生规则 {}：{}:{}；{}；任务 {}",
                finding["rule_id"],
                finding["path"],
                finding["line"],
                finding["rule_summary"],
                finding["finding_id"]
            );
        }
        println!(
            "原生未完成原因：{}；下一步：{}",
            report["native_report"]["incomplete_reasons"], report["next"]
        );
    }
    ExitCode::from(exit)
}
