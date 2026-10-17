//! 私有语法工作进程：只读 stdin 与固定内置 grammar，不解释项目文件或策略。

use crate::syntax_worker_envelope::SyntaxWorkerEnvelope;
use crate::syntax_worker_mode::SyntaxWorkerMode;
use crate::syntax_worker_recovery::SyntaxWorkerRecovery;
use codeguard_adapters::bundled_grammar_candidate;
use codeguard_runtime::{
    WasmGrammar, scan_wasm_empty_blocks, scan_wasm_recoveries, scan_wasm_root_child,
};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::ExitCode;

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const MAX_RECOVERIES: usize = 128;

/// 在独立进程中解析一份 stdin 源码；参数仅允许固定候选语种。
/// 返回原始恢复观察，不赋予 grammar 验收、原生 lint 或交付权威。
pub fn run(args: &[String]) -> ExitCode {
    let (language, mode) = match args {
        [language] => (language, SyntaxWorkerMode::Default),
        [language, option] if language == "javascript" && option == "--direct-bindings" => {
            (language, SyntaxWorkerMode::Bindings)
        }
        [language, option] if language == "erlang" && option == "--form-terminators" => {
            (language, SyntaxWorkerMode::Bindings)
        }
        [language, option] if language == "javascript" && option == "--javascript-module" => {
            (language, SyntaxWorkerMode::Module)
        }
        _ => {
            eprintln!("语法工作进程参数无效");
            return ExitCode::from(2);
        }
    };
    let mut source = Vec::new();
    match std::io::stdin()
        .lock()
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut source)
    {
        Ok(_) if source.len() as u64 > MAX_SOURCE_BYTES => {
            eprintln!("语法工作进程输入超过上限");
            return ExitCode::from(3);
        }
        Ok(_) => {}
        Err(_) => {
            eprintln!("语法工作进程输入读取失败");
            return ExitCode::from(4);
        }
    }
    if std::str::from_utf8(&source).is_err() {
        eprintln!("语法工作进程输入不是 UTF-8");
        return ExitCode::from(3);
    }
    match observe(language, &source, mode) {
        Ok(report) => match serde_json::to_string(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(_) => ExitCode::from(4),
        },
        Err(reason) => {
            eprintln!("语法工作进程未完成：{reason}");
            ExitCode::from(3)
        }
    }
}

fn observe(
    language: &str,
    source: &[u8],
    mode: SyntaxWorkerMode,
) -> Result<SyntaxWorkerEnvelope, String> {
    let bindings = mode != SyntaxWorkerMode::Default;
    let module = mode == SyntaxWorkerMode::Module;
    let (asset, wasm) = bundled_grammar_candidate(language)?;
    if asset.codeguard_runtime_validation != "rust_loader_smoke_passed" {
        return Err("grammar 与当前 Rust WASM 运行时不兼容".into());
    }
    let mut grammar = WasmGrammar::load(
        asset.loader_symbol.as_deref().unwrap_or(language),
        wasm,
        &asset.sha256,
        asset.abi_version as usize,
    )?;
    let tree = grammar.parse(source)?;
    let scanned = scan_wasm_recoveries(&tree, MAX_RECOVERIES)?;
    let mut structural_observations = Vec::new();
    let mut structural_truncated = false;
    if bindings && language == "javascript" {
        let kinds = codeguard_adapters::javascript_binding_node_kinds()?;
        let scanned_bindings = codeguard_runtime::scan_wasm_sibling_bindings(
            &tree,
            source,
            kinds.each_ref().map(String::as_str),
            MAX_RECOVERIES,
            200_000,
        )?;
        structural_truncated = scanned_bindings.truncated;
        for binding in scanned_bindings.duplicates {
            if scanned.recoveries.len() + structural_observations.len() == MAX_RECOVERIES {
                structural_truncated = true;
                break;
            }
            structural_observations.push(crate::syntax_worker_structure::SyntaxWorkerStructure {
                basis: "codeguard_structure_rule".into(),
                rule_id: "codeguard.javascript.duplicate_direct_lexical_binding".into(),
                rule_version: "1.0.0".into(),
                rule_sha256: codeguard_adapters::javascript_binding_rule_sha256(),
                parent_syntax_kind: binding.parent_syntax_kind,
                start_byte: binding.start_byte,
                end_byte: binding.end_byte,
                start_row: binding.start_row,
                start_column_byte: binding.start_column_byte,
                end_row: binding.end_row,
                end_column_byte: binding.end_column_byte,
            });
        }
    }
    if module {
        let (kinds, functions) = codeguard_adapters::javascript_module_return_node_kinds()?;
        let function_refs = functions.iter().map(String::as_str).collect::<Vec<_>>();
        let scanned_returns = codeguard_runtime::scan_wasm_outer_returns(
            &tree,
            source,
            kinds.each_ref().map(String::as_str),
            &function_refs,
            MAX_RECOVERIES,
            200_000,
        )?;
        structural_truncated |= scanned_returns.truncated;
        for fact in scanned_returns.returns {
            if scanned.recoveries.len() + structural_observations.len() == MAX_RECOVERIES {
                structural_truncated = true;
                break;
            }
            structural_observations.push(crate::syntax_worker_structure::SyntaxWorkerStructure {
                basis: "codeguard_structure_rule".into(),
                rule_id: "codeguard.javascript.module_return_outside_function".into(),
                rule_version: "1.0.0".into(),
                rule_sha256: codeguard_adapters::javascript_module_return_rule_sha256(),
                parent_syntax_kind: fact.syntax_kind,
                start_byte: fact.start_byte,
                end_byte: fact.end_byte,
                start_row: fact.start_row,
                start_column_byte: fact.start_column_byte,
                end_row: fact.end_row,
                end_column_byte: fact.end_column_byte,
            });
        }
    }
    if language == "python" {
        let blocks = scan_wasm_empty_blocks(&tree, MAX_RECOVERIES)?;
        structural_truncated = blocks.truncated;
        for block in blocks.blocks {
            if !codeguard_adapters::is_required_python_suite_parent(&block.parent_syntax_kind) {
                continue;
            }
            if scanned.recoveries.len() + structural_observations.len() == MAX_RECOVERIES {
                structural_truncated = true;
                break;
            }
            structural_observations.push(crate::syntax_worker_structure::SyntaxWorkerStructure {
                basis: "codeguard_structure_rule".into(),
                rule_id: "codeguard.python.required_suite".into(),
                rule_version: "1.0.0".into(),
                rule_sha256: codeguard_adapters::python_suite_rule_sha256(),
                parent_syntax_kind: block.parent_syntax_kind,
                start_byte: block.start_byte,
                end_byte: block.end_byte,
                start_row: block.start_row,
                start_column_byte: block.start_column_byte,
                end_row: block.end_row,
                end_column_byte: block.end_column_byte,
            });
        }
    }
    if language == "go" {
        // Go入口观察完整文件，片段模式不在此协议内；声明只能来自直接AST子节点。
        let root = scan_wasm_root_child(&tree, "package_clause", 200_000)?;
        match codeguard_adapters::missing_go_package_candidate(
            language,
            true,
            &root.root_syntax_kind,
            &root.child_syntax_kind,
            root.present,
            root.truncated,
        ) {
            Some(true) if scanned.recoveries.len() < MAX_RECOVERIES => {
                structural_observations.push(
                    crate::syntax_worker_structure::SyntaxWorkerStructure {
                        basis: "codeguard_structure_rule".into(),
                        rule_id: "codeguard.go.required_package".into(),
                        rule_version: "1.0.0".into(),
                        rule_sha256: codeguard_adapters::go_package_rule_sha256(),
                        parent_syntax_kind: root.root_syntax_kind,
                        // 缺整文件声明的零宽锚点，不冒充原生工具指出的错误列。
                        start_byte: 0,
                        end_byte: 0,
                        start_row: 0,
                        start_column_byte: 0,
                        end_row: 0,
                        end_column_byte: 0,
                    },
                );
            }
            Some(false) => {}
            Some(true) | None => structural_truncated = true,
        }
    }
    if language == "cfquery" && tree.root_node().kind() == "program" {
        let (facts, truncated) = codeguard_runtime::scan_wasm_keyword_sequence(
            &tree,
            source,
            "query_keyword",
            &["SELECT", "DISTINCT", "FROM"],
            MAX_RECOVERIES,
        )?;
        structural_truncated |= truncated;
        for fact in facts {
            if scanned.recoveries.len() + structural_observations.len() == MAX_RECOVERIES {
                structural_truncated = true;
                break;
            }
            structural_observations.push(crate::syntax_worker_structure::SyntaxWorkerStructure {
                basis: "codeguard_structure_rule".into(),
                rule_id: "codeguard.cfquery.distinct_projection".into(),
                rule_version: "1.0.0".into(),
                rule_sha256: codeguard_adapters::cfquery_projection_rule_sha256(),
                parent_syntax_kind: fact.root_syntax_kind,
                start_byte: fact.start_byte,
                end_byte: fact.end_byte,
                start_row: fact.start_row,
                start_column_byte: fact.start_column_byte,
                end_row: fact.end_row,
                end_column_byte: fact.end_column_byte,
            });
        }
    }
    if bindings && language == "erlang" && tree.root_node().kind() == "source_file" {
        let (facts, truncated) = codeguard_runtime::scan_wasm_form_terminators(
            &tree,
            "fun_decl",
            MAX_RECOVERIES,
            200_000,
        )?;
        structural_truncated |= truncated;
        for fact in facts {
            if scanned.recoveries.len() + structural_observations.len() == MAX_RECOVERIES {
                structural_truncated = true;
                break;
            }
            structural_observations.push(crate::syntax_worker_structure::SyntaxWorkerStructure {
                basis: "codeguard_structure_rule".into(),
                rule_id: "codeguard.erlang.form_terminator".into(),
                rule_version: "1.0.0".into(),
                rule_sha256: codeguard_adapters::erlang_form_rule_sha256(),
                parent_syntax_kind: fact.parent_syntax_kind,
                start_byte: fact.start_byte,
                end_byte: fact.end_byte,
                start_row: fact.start_row,
                start_column_byte: fact.start_column_byte,
                end_row: fact.end_row,
                end_column_byte: fact.end_column_byte,
            });
        }
    }
    let recoveries = scanned
        .recoveries
        .into_iter()
        .map(|anchor| SyntaxWorkerRecovery {
            kind: anchor.kind.into(),
            group_id: anchor.group_id,
            syntax_kind: anchor.syntax_kind,
            start_byte: anchor.start_byte,
            end_byte: anchor.end_byte,
            start_row: anchor.start_row,
            start_column_byte: anchor.start_column_byte,
            end_row: anchor.end_row,
            end_column_byte: anchor.end_column_byte,
        })
        .collect();
    Ok(SyntaxWorkerEnvelope {
        schema_version: if module {
            "1.7.0"
        } else if bindings && language == "erlang" && !structural_observations.is_empty() {
            "1.6.0"
        } else if bindings && !structural_observations.is_empty() {
            "1.5.0"
        } else if scanned.parser_error_location_unavailable {
            "1.4.0"
        } else if structural_observations.is_empty() {
            "1.0.0"
        } else if language == "cfquery" {
            "1.3.0"
        } else if language == "go" {
            "1.2.0"
        } else {
            "1.1.0"
        }
        .into(),
        report_type: "syntax_worker_candidate".into(),
        language: language.into(),
        grammar_sha256: asset.sha256.clone(),
        grammar_abi_version: asset.abi_version,
        source_sha256: format!("{:x}", Sha256::digest(source)),
        truncated: scanned.truncated || structural_truncated,
        parser_error_location_unavailable: scanned
            .parser_error_location_unavailable
            .then_some(true),
        javascript_mode: module.then(|| "module".into()),
        recoveries,
        structural_observations,
    })
}
