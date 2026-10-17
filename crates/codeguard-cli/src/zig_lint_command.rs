//! Zig 0.16 单文件原生 AST 检查优先；缺少可核对工具时才作候选 WASM 初检。

use crate::plain_syntax_source::read_plain_source;
#[cfg(feature = "wasm-precheck")]
use crate::syntax_worker_runner::run_syntax_worker_candidate;
use crate::zig_tool_selection::ZigToolSelection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
#[cfg(feature = "wasm-precheck")]
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

struct Args {
    source: PathBuf,
    zig_tool: Option<PathBuf>,
    json: bool,
}

/// 执行单文件 Zig 原生语法检查或未验收 WASM 初检，不签发完整 lint/交付通过。
/// 参数为源码文件、可选受控 Zig 二进制路径和输出格式；退出码 3 表示仍需完整验收。
pub fn run(args: &[String]) -> ExitCode {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    let selection = ZigToolSelection::discover(args.zig_tool.clone());
    let selected_target = selection.tool().and_then(|p| p.canonicalize().ok());
    let mut report = json!({
        "schema_version":"0.3.0", "report_type":"zig_lint_feedback",
        "tool_selection":selection.report(), "source_current":false,
        "operation":"lint", "language":"zig", "path":args.source,
        "status":"incomplete", "command_status":"incomplete", "exit_code":3,
        "coverage_proven":false,
        "delivery_decision":"not_evaluated", "authority":"local_unverified",
        "source_sha256":null,
        "native":{"status":"not_run","reason":"zig_tool_not_found_on_path","version":null,"tool_sha256":null,"diagnostics":[]},
        "syntax_precheck":null,
        "next_action":"提供适用的 Zig 0.16.0 原生工具并运行 ast-check；候选语法初检不能代替完整 lint、编译或测试"
    });
    // 取消先于任何工具或候选执行；不得在取消后继续启动新检查。
    if codeguard_runtime::sigint_cancellation_requested() {
        return cancelled(&mut report, args.json);
    }
    if args
        .source
        .extension()
        .is_none_or(|extension| extension != "zig")
    {
        report["native"]["reason"] = json!("zig_source_extension_required");
        emit(&report, args.json);
        return ExitCode::from(3);
    }
    let source = match read_plain_source(&args.source) {
        Ok(source) => source,
        Err(reason) => {
            report["native"]["reason"] = json!(reason);
            emit(&report, args.json);
            return ExitCode::from(3);
        }
    };
    report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source)));
    report["source_current"] = json!(true);
    if let Some(tool) = selection.tool() {
        if let Some(mut native) = selected_target
            .as_deref()
            .and_then(|frozen| observe_native(frozen, &source, deadline))
        {
            let source_current =
                read_plain_source(&args.source).is_ok_and(|current| current == source);
            report["source_current"] = json!(source_current);
            let target_current = tool.canonicalize().ok() == selected_target;
            if !source_current || !target_current {
                native["status"] = json!("incomplete");
                native["reason"] = json!(if !source_current {
                    "zig_source_changed_during_check"
                } else {
                    "zig_tool_target_changed_during_check"
                });
                native["diagnostics"] = json!([]);
                native["diagnostic_count"] = json!(0);
                report["native"] = native;
                report["next_action"] = json!(
                    "当前源码或选定工具入口已变化；保留检查阻塞，核对输入和原工具后重新检查，不沿用旧位置修复"
                );
                emit(&report, args.json);
                return ExitCode::from(3);
            }
            let completed = matches!(
                native["status"].as_str(),
                Some("completed" | "diagnostics_observed")
            );
            report["native"] = native;
            // 原生阶段被取消时不落入 WASM 候选初检；保留取消语义并返回 130。
            if report["native"]["reason"] == "request_cancelled"
                || codeguard_runtime::sigint_cancellation_requested()
            {
                return cancelled(&mut report, args.json);
            }
            if completed {
                report["next_action"] =
                    json!(if report["native"]["status"] == "diagnostics_observed" {
                        "核对原生 Zig AST 诊断的源码位置并修复；仍需执行项目完整 lint、构建和测试"
                    } else {
                        "Zig ast-check 未发现局部语法错误；仍需执行项目完整 lint、构建和测试"
                    });
                crate::native_syntax_confirmation::connect_file(
                    &args.source,
                    &mut report,
                    deadline,
                );
                emit(&report, args.json);
                return ExitCode::from(3);
            }
        } else {
            report["native"]["reason"] = json!("zig_tool_unavailable_or_untrusted");
        }
    }
    #[cfg(feature = "wasm-precheck")]
    {
        let name = args
            .source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("source.zig");
        match std::env::current_exe() {
            Ok(executable) => match run_syntax_worker_candidate(
                &executable,
                "zig",
                name,
                &source,
                deadline,
                &AtomicBool::new(false),
            ) {
                Ok(observation) => {
                    report["syntax_precheck"] = json!({
                        "status":"incomplete",
                        "grammar_qualified":false,
                        "grammar_sha256":observation.grammar_sha256,
                        "precheck":observation.precheck,
                        "recoveries":observation.recoveries
                    });
                    report["next_action"] = json!(if report["syntax_precheck"]["recoveries"]
                        .as_array()
                        .is_some_and(|items| !items.is_empty())
                    {
                        "先用匹配 Zig 版本的原生语法检查确认疑似位置；不要仅凭 WASM 修改源码或关闭任务"
                    } else {
                        "候选语法初检未见恢复节点；仍需配置并运行原生 Zig 检查"
                    });
                }
                Err(reason) => {
                    report["syntax_precheck"] = json!({"status":"incomplete","reason":reason});
                }
            },
            Err(_) => {
                report["syntax_precheck"] =
                    json!({"status":"incomplete","reason":"worker_executable_unavailable"});
            }
        }
    }
    #[cfg(not(feature = "wasm-precheck"))]
    {
        report["syntax_precheck"] =
            json!({"status":"incomplete","reason":"wasm_feature_not_built"});
    }
    if read_plain_source(&args.source).is_ok_and(|current| current == source) {
        report["source_current"] = json!(true);
    } else {
        report["source_current"] = json!(false);
        report["native"]["status"] = json!("incomplete");
        report["native"]["reason"] = json!("zig_source_changed_during_precheck");
        report["native"]["diagnostics"] = json!([]);
        report["native"]["diagnostic_count"] = json!(0);
        report["syntax_precheck"] = Value::Null;
        report["next_action"] =
            json!("候选初检期间源码变化；重新读取当前源码和原工具，不沿用旧观察");
    }
    if selection.tool().is_some() && report["source_current"] == true {
        crate::native_syntax_confirmation::connect_file(&args.source, &mut report, deadline);
    }
    // 候选初检期间收到的取消同样不签发任何通过结论。
    if codeguard_runtime::sigint_cancellation_requested() {
        return cancelled(&mut report, args.json);
    }
    emit(&report, args.json);
    ExitCode::from(3)
}

/// 记录取消终态并返回 130；不缓存 clean，不把取消解释成检查通过或工具缺陷。
fn cancelled(report: &mut Value, json_format: bool) -> ExitCode {
    report["status"] = json!("cancelled");
    report["command_status"] = json!("cancelled");
    report["exit_code"] = json!(130);
    report["coverage_proven"] = json!(false);
    report["delivery_decision"] = json!("not_evaluated");
    report["next_action"] = json!(
        "请求已取消；保留取消状态，不缓存也不签发 clean；需要时以同一工具和输入重新执行原检查"
    );
    emit(report, json_format);
    ExitCode::from(130)
}

fn observe_native(tool: &Path, source: &[u8], deadline: Instant) -> Option<Value> {
    crate::zig_syntax_probe::observe(tool, source, &std::env::current_dir().ok()?, deadline)
}

fn parse_args(args: &[String]) -> Result<Args, &'static str> {
    let mut source = None;
    let mut zig_tool = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--zig-tool" => {
                index += 1;
                let value = args.get(index).ok_or("--zig-tool 缺少路径")?;
                if zig_tool.replace(PathBuf::from(value)).is_some() {
                    return Err("--zig-tool 重复");
                }
            }
            "--format=json" => json = true,
            "--format=human" => json = false,
            value if !value.starts_with('-') && source.is_none() => {
                source = Some(PathBuf::from(value))
            }
            _ => return Err("lint zig 参数无效"),
        }
        index += 1;
    }
    if zig_tool.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--zig-tool 必须是绝对路径");
    }
    Ok(Args {
        source: source.ok_or("lint zig 缺少源码文件")?,
        zig_tool,
        json,
    })
}

fn emit(report: &Value, json_format: bool) {
    if json_format {
        println!("{report}");
    } else {
        if report["command_status"] == "cancelled" {
            println!("Zig 检查已取消（退出 130）；不缓存 clean，不签发任何通过结论");
        }
        println!(
            "Zig 局部检查未完成：原生 {}，候选语法 {}；{}",
            report["native"]["status"].as_str().unwrap_or("incomplete"),
            report["syntax_precheck"]["status"]
                .as_str()
                .unwrap_or("未运行"),
            report["next_action"]
                .as_str()
                .unwrap_or("核对原生 Zig 结果")
        );
        if let Some(id) = report["task_id"].as_str() {
            println!(
                "原生任务 {id}；读取 codeguard task show {id} . --format=json，修复后用原工具 task verify"
            );
        }
        for diagnostic in report["native"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "  原生 AST 错误：{}:{}",
                diagnostic["line"], diagnostic["column"]
            );
        }
    }
}
