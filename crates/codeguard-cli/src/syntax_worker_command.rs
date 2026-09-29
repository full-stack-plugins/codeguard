//! 私有语法工作进程：只读 stdin 与固定内置 grammar，不解释项目文件或策略。

use crate::syntax_worker_envelope::SyntaxWorkerEnvelope;
use crate::syntax_worker_recovery::SyntaxWorkerRecovery;
use codeguard_adapters::bundled_grammar_candidate;
use codeguard_runtime::{WasmGrammar, scan_wasm_recoveries};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::ExitCode;

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const MAX_RECOVERIES: usize = 128;

/// 在独立进程中解析一份 stdin 源码；参数仅允许固定候选语种。
/// 返回原始恢复观察，不赋予 grammar 验收、原生 lint 或交付权威。
pub fn run(args: &[String]) -> ExitCode {
    let [language] = args else {
        eprintln!("语法工作进程参数无效");
        return ExitCode::from(2);
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
    match observe(language, &source) {
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

fn observe(language: &str, source: &[u8]) -> Result<SyntaxWorkerEnvelope, String> {
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
        schema_version: "1.0.0".into(),
        report_type: "syntax_worker_candidate".into(),
        language: language.into(),
        grammar_sha256: asset.sha256.clone(),
        grammar_abi_version: asset.abi_version,
        source_sha256: format!("{:x}", Sha256::digest(source)),
        truncated: scanned.truncated,
        recoveries,
    })
}
