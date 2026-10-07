//! Rust/Python/TypeScript/JavaScript grammar 扩展语料精度验证。
//!
//! 直接用 WASM grammar 解析各语言样本，提取 TP/FP/FN/TN 计数并生成 PrecisionValidation 证据。
//! 只在 wasm-precheck 特性下运行。

#![cfg(feature = "wasm-precheck")]

use codeguard_adapters::generate_precision_validation;
use serde_json::Value;
use sha2::Digest;
use std::path::Path;

fn classify_from_grammar(grammar: &mut codeguard_runtime::WasmGrammar, source: &[u8]) -> Option<bool> {
    match grammar.parse(source) {
        Ok(tree) => {
            let mut has_error = false;
            let mut cursor = tree.walk();
            loop {
                let node = cursor.node();
                if node.kind() == "ERROR" || node.is_missing() {
                    has_error = true;
                    break;
                }
                if !cursor.goto_next_sibling() && !cursor.goto_first_child() {
                    break;
                }
            }
            Some(!has_error)
        }
        Err(_) => None,
    }
}

fn validate_language(language: &str, wasm_file: &str, abi: usize) {
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..").join("tests").join("fixtures")
        .join(format!("{language}_grammar_corpus_expanded.json"));
    let corpus_bytes = std::fs::read(&corpus_path).unwrap_or_else(|_| panic!("{language} 语料文件不存在"));
    let corpus: Value = serde_json::from_slice(&corpus_bytes).expect("语料 JSON 无效");
    let cases = corpus["cases"].as_array().expect("cases 字段缺失");

    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..").join("grammars").join(wasm_file);
    let wasm_bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| panic!("{wasm_file} WASM 文件不存在"));
    let wasm_sha256 = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));
    let mut grammar = codeguard_runtime::WasmGrammar::load(language, &wasm_bytes, &wasm_sha256, abi)
        .unwrap_or_else(|e| panic!("{language} grammar 加载失败: {e}"));

    let mut tp = 0u32;
    let mut fp = 0u32;
    let mut fn_count = 0u32;
    let mut tn = 0u32;
    let mut unknown = 0u32;

    for case in cases {
        let expected_valid = case["expected_valid"].as_bool().unwrap();
        let source = case["source"].as_str().unwrap();
        let classification = classify_from_grammar(&mut grammar, source.as_bytes());
        match (expected_valid, classification) {
            (false, Some(false)) => tp += 1,
            (true, Some(false)) => fp += 1,
            (false, Some(true)) => fn_count += 1,
            (true, Some(true)) => tn += 1,
            _ => unknown += 1,
        }
    }

    let total = cases.len() as u32;
    println!("\n=== {language} grammar 精度验证 ===");
    println!("总样本: {total}");
    println!("TP={tp} FP={fp} FN={fn_count} TN={tn} unknown={unknown}");
    println!("精度 = TP/(TP+FP) = {}", if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 });

    let evidence = generate_precision_validation(
        "2026-10-07", total, tp, fp, fn_count, tn, unknown,
        &format!("tests/acceptance/{language}-grammar-precision-2026-10-07.md"),
        None,
    );

    match evidence {
        Ok(ev) => {
            println!("✅ {language} PASS! Wilson 下界 = {:.4}", ev.precision_wilson_lower_bound);
            // 写入验收文档
            let doc = format!(
                "# {language} Grammar 精度验证证据\n\n\
                 > 日期：2026-10-07；OpenSpec 14.17 / 15.2。\n\
                 > 语料：{total} 个语法样本。\n\n\
                 | 指标 | 值 |\n|---|---|\n\
                 | 总样本 | {total} |\n\
                 | TP | {tp} |\n| FP | {fp} |\n| FN | {fn_count} |\n| TN | {tn} |\n| unknown | {unknown} |\n\
                 | Wilson 下界 (95%) | {:.4} |\n\
                 | 结果 | **PASS** |\n",
                ev.precision_wilson_lower_bound
            );
            let doc_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..").join("..").join("tests").join("acceptance")
                .join(format!("{language}-grammar-precision-2026-10-07.md"));
            std::fs::write(&doc_path, doc).expect("无法写入验收文档");
        }
        Err(e) => {
            println!("❌ {language} FAIL: {e}");
            panic!("{language} 精度验证未通过: {e}");
        }
    }
}

#[test]
fn rust_grammar_precision() {
    validate_language("rust", "rust/parser.wasm", 15);
}

#[test]
fn python_grammar_precision() {
    validate_language("python", "python/parser.wasm", 14);
}

#[test]
fn typescript_grammar_precision() {
    validate_language("typescript", "typescript/parser.wasm", 14);
}

#[test]
fn javascript_grammar_precision() {
    validate_language("javascript", "javascript/parser.wasm", 15);
}
