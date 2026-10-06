//! 显式单文件候选语法观察；不将未经验收的 grammar 结果变为 lint 结论。

use crate::syntax_worker_runner::{
    run_syntax_worker_candidate, run_syntax_worker_binding_candidate,
    run_syntax_worker_form_candidate, run_syntax_worker_module_candidate,
};
use codeguard_adapters::bundled_grammar_candidate;
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// 对固定语言和普通 UTF-8 文件执行隔离候选解析，始终返回未完成。
/// 参数为语言、源码文件、仅JavaScript的显式 `--module` 和可选 `--format=json`；返回3要求原生确认。
pub fn run(args: &[String]) -> ExitCode {
    let (language, source_path, module) = match args {
        [language, source] => (language, source, false),
        [language, source, format] if format == "--format=json" => (language, source, false),
        [language, source, option] if language == "javascript" && option == "--module" => {
            (language, source, true)
        }
        [language, source, option, format]
            if language == "javascript" && option == "--module" && format == "--format=json" =>
        {
            (language, source, true)
        }
        _ => {
            eprintln!(
                "用法: grammar probe <language> <file> [--module（仅JavaScript）] [--format=json]"
            );
            return ExitCode::from(2);
        }
    };
    if let Err(reason) = bundled_grammar_candidate(language) {
        if reason == "不支持的 grammar 语种" {
            eprintln!("不支持的 grammar 语种");
            return ExitCode::from(2);
        }
        return emit_incomplete(language, source_path, &reason, module);
    }
    let path = PathBuf::from(source_path);
    let source = match read_plain_source(&path) {
        Ok(source) => source,
        Err(reason) => return emit_incomplete(language, source_path, reason, module),
    };
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return ExitCode::from(4),
    };
    let deadline = Instant::now() + Duration::from_secs(90);
    let relative_name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name,
        None => return emit_incomplete(language, source_path, "source_path_invalid", module),
    };
    let runner = if module {
        run_syntax_worker_module_candidate
    } else if language == "javascript" {
        run_syntax_worker_binding_candidate
    } else if language == "erlang" {
        run_syntax_worker_form_candidate
    } else {
        run_syntax_worker_candidate
    };
    let result = runner(
        &executable,
        language,
        relative_name,
        &source,
        deadline,
        &AtomicBool::new(false),
    );
    match result {
        Ok(observation) => {
            let mut report = json!({
                "schema_version":"0.1.0",
                "report_type":"grammar_candidate_probe",
                "status":"incomplete",
                "language":language,
                "path":source_path,
                "source_sha256":observation.source_sha256,
                "grammar_sha256":observation.grammar_sha256,
                "grammar_qualified":false,
                "precheck":observation.precheck,
                "recoveries":observation.recoveries,
                "native":{"status":"not_run","reason":"explicit_candidate_probe"},
                "delivery_decision":"not_evaluated",
                "next_action":if observation.recoveries.is_empty() && observation.structural_observations.is_empty() {
                    "run_or_configure_applicable_native_lint_before_delivery"
                } else {
                    "confirm_suspected_recoveries_with_applicable_native_tool"
                }
            });
            if !observation.structural_observations.is_empty() {
                report["schema_version"] = json!(if language == "erlang" {
                    "0.7.0"
                } else if language == "javascript" {
                    "0.6.0"
                } else if language == "cfquery" {
                    "0.4.0"
                } else if language == "go" {
                    "0.3.0"
                } else {
                    "0.2.0"
                });
                if language == "cfquery" {
                    report["source_scope"] = json!("static_cfquery_sql_candidate");
                }
                if language == "javascript" {
                    report["structural_rule_scope"] = json!("direct_simple_lexical_names_only");
                }
                if language == "erlang" {
                    report["source_scope"] = json!("direct_function_forms");
                }
                if language == "go" {
                    report["source_scope"] = json!("whole_file");
                }
                report["precheck_scope"] = json!("raw_parser_recoveries_and_observation_budgets");
                report["next_action"] =
                    json!("confirm_candidate_structure_with_applicable_native_tool");
                report["structural_observations"] = json!(observation.structural_observations);
            }
            if observation.parser_error_location_unavailable {
                if report["schema_version"] != "0.6.0" && report["schema_version"] != "0.7.0" {
                    report["schema_version"] = json!("0.5.0");
                }
                report["parser_error_location_unavailable"] = json!(true);
                report["next_action"] =
                    json!("compare_original_source_with_native_tool_then_review_grammar");
            }
            if module {
                report["schema_version"] = json!("0.8.0");
                report["javascript_mode"] = json!("module");
                report["structural_rule_scope"] =
                    json!("direct_simple_lexical_names_and_explicit_module_returns");
                report["precheck_scope"] = json!("raw_parser_recoveries_and_observation_budgets");
                report["structural_observations"] = json!(observation.structural_observations);
            }
            println!("{report}");
            ExitCode::from(3)
        }
        Err(reason) => emit_incomplete(language, source_path, &reason, module),
    }
}

fn emit_incomplete(language: &str, source_path: &str, reason: &str, module: bool) -> ExitCode {
    let mut report = json!({
        "schema_version":if module {"0.8.0"}else{"0.1.0"},
        "report_type":"grammar_candidate_probe", "language":language, "path":source_path,
        "status":"incomplete", "reason":reason,
        "native":{"status":"not_run","reason":"explicit_candidate_probe"},
        "delivery_decision":"not_evaluated"
    });
    if module {
        report["javascript_mode"] = json!("module");
    }
    println!("{report}");
    ExitCode::from(3)
}

pub(crate) use crate::plain_syntax_source::read_plain_source;
