//! Zig 0.16 单文件原生 AST 检查优先；缺少可核对工具时才作候选 WASM 初检。

use crate::plain_syntax_source::read_plain_source;
#[cfg(feature = "wasm-precheck")]
use crate::syntax_worker_runner::run_syntax_worker_candidate;
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
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"zig_lint_feedback",
        "operation":"lint", "language":"zig", "path":args.source,
        "status":"incomplete", "coverage_proven":false,
        "delivery_decision":"not_evaluated", "authority":"local_unverified",
        "source_sha256":null,
        "native":{"status":"not_run","reason":"explicit_zig_tool_not_provided","version":null,"tool_sha256":null,"diagnostics":[]},
        "syntax_precheck":null,
        "next_action":"提供适用的 Zig 0.16.0 原生工具并运行 ast-check；候选语法初检不能代替完整 lint、编译或测试"
    });
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
    if let Some(tool) = args.zig_tool.as_deref() {
        if let Some(native) = observe_native(tool, &source, deadline) {
            let completed = matches!(
                native["status"].as_str(),
                Some("completed" | "diagnostics_observed")
            );
            report["native"] = native;
            if completed {
                report["next_action"] =
                    json!(if report["native"]["status"] == "diagnostics_observed" {
                        "核对原生 Zig AST 诊断的源码位置并修复；仍需执行项目完整 lint、构建和测试"
                    } else {
                        "Zig ast-check 未发现局部语法错误；仍需执行项目完整 lint、构建和测试"
                    });
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
    emit(&report, args.json);
    ExitCode::from(3)
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
