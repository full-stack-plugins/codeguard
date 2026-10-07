//! C# grammar 扩展语料精度验证。

#![cfg(feature = "wasm-precheck")]

use codeguard_adapters::generate_precision_validation;
use serde_json::Value;
use sha2::Digest;
use std::path::Path;

fn classify_from_grammar(
    grammar: &mut codeguard_runtime::WasmGrammar,
    source: &[u8],
) -> Option<bool> {
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

#[test]
fn csharp_grammar_precision() {
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("csharp_grammar_corpus_expanded.json");
    let corpus_bytes = std::fs::read(&corpus_path).expect("csharp 语料文件不存在");
    let corpus: Value = serde_json::from_slice(&corpus_bytes).expect("语料 JSON 无效");
    let cases = corpus["cases"].as_array().expect("cases 字段缺失");

    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("grammars")
        .join("csharp")
        .join("parser.wasm");
    let wasm_bytes = std::fs::read(&wasm_path).expect("csharp WASM 文件不存在");
    let wasm_sha256 = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));
    let mut grammar =
        codeguard_runtime::WasmGrammar::load("c_sharp", &wasm_bytes, &wasm_sha256, 15)
            .expect("csharp grammar 加载失败");

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

    println!("\n=== csharp grammar 精度验证 ===");
    println!("总样本: {}", cases.len());
    println!("TP={tp} FP={fp} FN={fn_count} TN={tn} unknown={unknown}");

    let evidence = generate_precision_validation(
        "2026-10-07",
        cases.len() as u32,
        tp,
        fp,
        fn_count,
        tn,
        unknown,
        "tests/acceptance/csharp-grammar-precision-2026-10-07.md",
        None,
    );

    match evidence {
        Ok(ev) => {
            println!(
                "✅ csharp PASS! Wilson 下界 = {:.4}",
                ev.precision_wilson_lower_bound
            );
            let doc = format!(
                "# C# Grammar 精度验证证据\n\n> 日期：2026-10-07。\n\n| 指标 | 值 |\n|---|---|\n| 总样本 | {} |\n| TP | {tp} |\n| FP | {fp} |\n| FN | {fn_count} |\n| TN | {tn} |\n| Wilson 下界 (95%) | {:.4} |\n| 结果 | **PASS** |\n",
                cases.len(),
                ev.precision_wilson_lower_bound
            );
            std::fs::write(
                "tests/acceptance/csharp-grammar-precision-2026-10-07.md",
                doc,
            )
            .unwrap();
        }
        Err(e) => panic!("csharp 精度验证未通过: {e}"),
    }
}
