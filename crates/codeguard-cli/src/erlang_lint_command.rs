//! Erlang 原生单文件 forms 解析优先，工具未提供时附加未验收 WASM 初检。

use crate::{erlang_lint_arguments::ErlangLintArguments, plain_syntax_source::read_plain_source};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    process::ExitCode,
    time::{Duration, Instant},
};

/// 执行普通 Erlang 源码的有界原生观察或固定候选初检。
/// 参数为源码、显式 erl 与格式/预算；整体仍为未完成，取消返回 130。
pub fn run(args: &[String]) -> ExitCode {
    let args = match ErlangLintArguments::parse(args) {
        Ok(args) => args,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(args.timeout.0);
    if codeguard_runtime::install_sigint_cancellation().is_err() {
        eprintln!("Erlang 检查无法登记取消信号");
        return ExitCode::from(4);
    }
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"erlang_lint_feedback",
        "operation":"lint", "language":"erlang", "path":args.source,
        "status":"incomplete", "coverage_proven":false, "delivery_decision":"not_evaluated",
        "authority":"local_unverified", "scope":"single_file_forms_without_preprocessing",
        "source_sha256":null, "execution_budget":crate::check_budget::budget_record(args.timeout.0,args.timeout.1),
        "native":{"status":"not_run","reason":"explicit_erl_tool_not_provided",
            "version":null,"tool_sha256":null,"diagnostics":[],"diagnostics_truncated":false,"preprocessing_unresolved":false},
        "syntax_precheck":null,
        "known_limitations":codeguard_adapters::bundled_grammar_candidate("erlang").ok().map(|(asset,_)|asset.known_limitations.clone()).unwrap_or_default(),
        "next_action":"提供 OTP 28 工具并执行 codeguard lint erlang FILE --erl-tool ABS_PATH；还需项目原生 lint、编译和测试"
    });
    let checked = (|| {
        if !args
            .source
            .extension()
            .is_some_and(|suffix| suffix == "erl" || suffix == "hrl")
        {
            return Err("erlang_source_extension_required");
        }
        let source = read_plain_source(&args.source)?;
        report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source)));
        if let Some(tool) = args.erl_tool.as_deref() {
            report["native"] = crate::erlang_syntax_probe::observe(tool, &source, deadline);
            report["next_action"] = json!(match report["native"]["status"].as_str() {
                Some("diagnostics_observed") =>
                    "核对并修复原生 Erlang 语法诊断，再执行同一 --erl-tool 命令；还需项目完整 lint、编译和测试",
                Some("completed") =>
                    "原生 Erlang 单文件 forms 解析没有诊断；还需项目完整 lint、预处理、编译和测试",
                _ =>
                    "先解决原生工具、预处理或执行阻塞，再执行同一 --erl-tool 命令；不要据此反复修改无关源码",
            });
        } else {
            #[cfg(feature = "wasm-precheck")]
            {
                use std::sync::atomic::AtomicBool;
                let observation = std::env::current_exe()
                    .map_err(|_| "worker_executable_unavailable")
                    .and_then(|executable| {
                        crate::syntax_worker_runner::run_syntax_worker_candidate(
                            &executable,
                            "erlang",
                            args.source
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("source.erl"),
                            &source,
                            deadline,
                            &AtomicBool::new(false),
                        )
                        .map_err(|_| "erlang_syntax_worker_incomplete")
                    });
                report["syntax_precheck"] = match observation {
                    Ok(observation) => json!({"status":"incomplete","grammar_qualified":false,
                        "grammar_sha256":observation.grammar_sha256,"precheck":observation.precheck,"recoveries":observation.recoveries}),
                    Err(reason) => json!({"status":"incomplete","reason":reason}),
                };
            }
            #[cfg(not(feature = "wasm-precheck"))]
            {
                report["syntax_precheck"] =
                    json!({"status":"incomplete","reason":"wasm_feature_not_built"});
            }
        }
        if read_plain_source(&args.source).ok().as_deref() != Some(source.as_slice()) {
            report["native"]["status"] = json!("incomplete");
            report["native"]["reason"] = json!("erlang_source_changed_during_check");
            report["native"]["diagnostics"] = json!([]);
            report["syntax_precheck"] = Value::Null;
            report["next_action"] = json!("源码字节已变化，请对当前字节重新检查");
        }
        Ok(())
    })();
    if let Err(reason) = checked {
        report["native"]["reason"] = json!(reason);
    }
    let cancelled = codeguard_runtime::sigint_cancellation_requested();
    if cancelled {
        report["native"]["status"] = json!("incomplete");
        report["native"]["reason"] = json!("request_cancelled");
        report["next_action"] = json!("检查已取消；需要重新运行以取得当前原生结果");
    }
    emit(&report, args.json);
    ExitCode::from(if cancelled { 130 } else { 3 })
}

fn emit(report: &Value, json_format: bool) {
    if json_format {
        println!("{report}");
        return;
    }
    let conclusion = match report["native"]["status"].as_str() {
        Some("completed") => "Erlang 单文件原生语法未见错误；完整项目检查仍待执行",
        Some("diagnostics_observed") => "Erlang 单文件原生语法发现诊断；先修复再复检",
        Some("not_run") if !report["syntax_precheck"].is_null() => {
            "Erlang 候选语法初检待原生确认；完整项目检查尚未完成"
        }
        _ => "Erlang 原生检查或覆盖待恢复；不能据此确认源码违规",
    };
    println!(
        "{}（{}）；{}",
        conclusion,
        report["native"]["reason"].as_str().unwrap_or("unresolved"),
        report["next_action"].as_str().unwrap_or("运行适用原生工具")
    );
    if !report["syntax_precheck"].is_null() {
        println!("  内置 grammar 已知可能漏掉函数末尾句点；即使零恢复节点，也需原生确认");
    }
    for diagnostic in report["native"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
    {
        println!(
            "  原生语法错误：{}:{}",
            diagnostic["line"], diagnostic["column"]
        );
    }
}
