//! 注册表语言的局部候选 lint 入口；原生适配缺口不被误称为未安装。
use crate::syntax_lint_arguments::SyntaxLintArguments;
#[cfg(feature = "wasm-precheck")]
use serde_json::Value;
use serde_json::json;
#[cfg(feature = "wasm-precheck")]
use std::collections::BTreeSet;
use std::{
    process::ExitCode,
    time::{Duration, Instant},
};

/// 为注册表语言返回明确Clang原生观察或局部候选与修复任务；不执行注册表历史命令。
/// 参数只允许明确文件与预算/工作区/格式；返回3或取消130，绝不签发通过。
pub fn run(args: &[String]) -> ExitCode {
    let args = match SyntaxLintArguments::parse(args) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(args.timeout_ms);
    let mut report = json!({"schema_version":"0.1.0","report_type":"syntax_lint_feedback","selection":{"category":"lint","language":args.language},
        "status":"incomplete","coverage_proven":false,"delivery_decision":"not_evaluated","findings":[],
        "native":{"status":"not_run","reason":"native_adapter_not_integrated","configuration_status":"unknown"},
        "syntax_candidates":null,"syntax_tasks":null,"setup":{"requirement":"required","task_id":null},
        "reason":"wasm_not_enabled","next_action":"此语言的原生lint适配尚未接入；核对项目实际原生工具与配置，补齐适用适配或提出具体能力决策。本次不证明工具未安装，不运行历史注册表命令或自动安装，不授予交付通过"});
    if args.clang_tool.is_some() {
        report = crate::clang_lint_feedback::observe(&args, deadline);
    } else {
        #[cfg(feature = "wasm-precheck")]
        observe(&args, deadline, &mut report);
    }
    #[cfg(not(feature = "wasm-precheck"))]
    let _ = deadline;
    if codeguard_runtime::sigint_cancellation_requested() {
        report["reason"] = json!("request_cancelled");
    }
    if args.json {
        println!("{report}");
    } else {
        if args.clang_tool.is_some() {
            println!(
                "{} lint：固定Clang原生语法观察；完整项目检查未完成。",
                args.language
            );
            for row in report["native"]["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .take(8)
            {
                println!(
                    "原生规则 {}，一开始行 {}、UTF-8字节列 {}",
                    row["rule_id"], row["line"], row["column_byte"]
                );
            }
            println!(
                "原生状态 {}；原因 {}",
                report["native"]["status"], report["native"]["reason"]
            );
        } else {
            println!(
                "{} lint：检查未完成；原生适配器尚未接入，工具配置状态未知。",
                args.language
            );
            #[cfg(feature = "wasm-precheck")]
            crate::syntax_lint_feedback::print_feedback(&report["syntax_candidates"]);
        }
        println!(
            "{}",
            report["next_action"].as_str().unwrap_or("需要原生确认")
        );
    }
    if report["reason"] == "request_cancelled" {
        ExitCode::from(130)
    } else {
        ExitCode::from(3)
    }
}

#[cfg(feature = "wasm-precheck")]
fn observe(args: &SyntaxLintArguments, deadline: Instant, report: &mut Value) {
    let bytes = match crate::plain_syntax_source::read_plain_source(&args.source) {
        Ok(bytes) => bytes,
        Err(reason) => {
            report["reason"] = json!(reason);
            return;
        }
    };
    let Ok(source) = args.source.canonicalize() else {
        report["reason"] = json!("source_scope_unavailable");
        return;
    };
    let root = match args.workspace.as_deref().or_else(|| source.parent()) {
        Some(root) => match root.canonicalize() {
            Ok(path) if args.workspace.as_ref().is_none_or(|r| *r == path) => path,
            _ => {
                report["reason"] = json!("source_scope_unavailable");
                return;
            }
        },
        None => {
            report["reason"] = json!("source_scope_unavailable");
            return;
        }
    };
    let Some(relative) = source.strip_prefix(&root).ok().and_then(|p| p.to_str()) else {
        report["reason"] = json!("source_scope_unavailable");
        return;
    };
    let routes = crate::grammar_route::route_source(relative, &bytes);
    let applicable = |language: &str| {
        language == args.language
            || (args.language == "cfml" && matches!(language, "cfscript" | "cfquery"))
    };
    if routes.is_empty() || routes.iter().any(|route| !applicable(route.language)) {
        report["reason"] = json!("selected_language_or_grammar_scope_unresolved");
        return;
    }
    let empty = Value::Null;
    let coverage = crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage::default();
    let syntax = crate::check_syntax_candidates::observe_selected(
        &root,
        &BTreeSet::from([relative.to_owned()]),
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &empty,
            python_lint: &empty,
            go_lint: &empty,
            erlang_lint: &empty,
            kotlin_lint: &empty,
            zig_lint: &empty,
            swift_lint: &empty,
            ruby_lint: &empty,
            rust_targets: &coverage,
            go_tool: None,
        },
        1,
        deadline,
        None,
    );
    report["reason"] = json!("native_adapter_not_integrated");
    if args.workspace.is_some() && !codeguard_runtime::sigint_cancellation_requested() {
        let mut tasks = crate::syntax_confirmation::persist(&root, &syntax, deadline);
        let mut selected = tasks["tasks"][0]["task_id"].as_str().map(str::to_owned);
        if selected.is_none() {
            for route in &routes {
                match crate::syntax_lint_feedback::pending_confirmation(
                    &root,
                    relative,
                    route.language,
                ) {
                    Ok(Some(id)) => {
                        selected = Some(id);
                        break;
                    }
                    Ok(None) => {}
                    Err(reason) => {
                        tasks["status"] = json!("incomplete");
                        tasks["failures"]
                            .as_array_mut()
                            .expect("同步结果包含失败数组")
                            .push(
                                json!({"path":relative,"language":route.language,"reason":reason}),
                            );
                    }
                }
            }
        }
        if let Some(id) = selected {
            report["setup"]["task_id"] = json!(id);
            if let Ok(brief) = crate::next_command::read_task_brief(&root, &id) {
                report["next_action"] = brief["step"].clone();
            }
        }
        if tasks["status"] == "incomplete" {
            report["next_action"] = json!(if tasks["failures"].as_array().is_some_and(|rows| rows
                .iter()
                .any(|r| r["reason"] == "workspace_not_initialized"))
            {
                "工作区未初始化；执行 codeguard init <原工作区> --apply，再按原单文件lint命令复检。不修改无关源码或虚构任务"
            } else {
                "查看 syntax_tasks.failures，恢复原工作区受管身份及事实/任务/报告记录；保留原生确认要求，不修改无关源码或伪造关闭"
            });
        }
        report["syntax_tasks"] = tasks;
    }
    report["syntax_candidates"] = syntax;
}
