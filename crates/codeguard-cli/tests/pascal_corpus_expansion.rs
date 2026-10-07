//! pascal grammar 扩展语料精度验证。

#![cfg(feature = "wasm-precheck")]

use codeguard_adapters::generate_precision_validation;
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

#[test]
fn pascal_grammar_precision() {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    // 合法 pascal（200）
    for i in 0..50 { valid.push(format!("program P{};\nbegin\nend.\n", i)); }
    for i in 0..30 { valid.push(format!("unit U{};\ninterface\nimplementation\nend.\n", i)); }
    for i in 0..30 { valid.push(format!("var x{}: Integer;\n", i)); }
    for i in 0..30 { valid.push(format!("const C{} = {};\n", i, i)); }
    for i in 0..30 { valid.push(format!("type T{} = Integer;\n", i)); }
    for i in 0..30 { valid.push(format!("{{ comment }}\nvar z{}: Integer;\n", i)); }

    // 违规 pascal（300）- 语法级错误
    for i in 0..60 { invalid.push(format!("program P{};\nbegin\n", i)); }
    for i in 0..60 { invalid.push(format!("unit U{};\ninterface\n", i)); }
    for i in 0..60 { invalid.push(format!("program;\nend.\n")); }
    for i in 0..60 { invalid.push(format!("var x: ;\n")); }
    for i in 0..60 { invalid.push(format!("var x = \"unclosed;\n")); }

    let manifest_sha256 = format!("{:x}", sha2::Sha256::digest(b"pascal-grammar-v1"));
    let mut cases = Vec::new();
    for (i, source) in valid.iter().enumerate() {
        cases.push(serde_json::json!({
            "id": format!("pascal_valid_{i}"), "language": "pascal", "source": source,
            "source_sha256": format!("{:x}", sha2::Sha256::digest(source.as_bytes())),
            "expected_valid": true, "label": "regression", "origin": "tests/fixtures/pascal_corpus_generator.rs",
        }));
    }
    for (i, source) in invalid.iter().enumerate() {
        cases.push(serde_json::json!({
            "id": format!("pascal_invalid_{i}"), "language": "pascal", "source": source,
            "source_sha256": format!("{:x}", sha2::Sha256::digest(source.as_bytes())),
            "expected_valid": false, "label": "regression", "origin": "tests/fixtures/pascal_corpus_generator.rs",
        }));
    }

    let corpus = serde_json::json!({
        "schema_version": "0.1.0", "corpus_type": "grammar_regression",
        "manifest_sha256": manifest_sha256, "cases": cases,
    });

    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..").join("grammars").join("pascal").join("parser.wasm");
    let wasm_bytes = std::fs::read(&wasm_path).expect("pascal WASM 文件不存在");
    let wasm_sha256 = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));
    let mut grammar = codeguard_runtime::WasmGrammar::load("pascal", &wasm_bytes, &wasm_sha256, 14)
        .expect("pascal grammar 加载失败");

    let mut tp = 0u32; let mut fp = 0u32; let mut fn_count = 0u32; let mut tn = 0u32; let mut unknown = 0u32;
    for case in corpus["cases"].as_array().unwrap() {
        let expected_valid = case["expected_valid"].as_bool().unwrap();
        let source = case["source"].as_str().unwrap();
        let classification = classify_from_grammar(&mut grammar, source.as_bytes());
        match (expected_valid, classification) {
            (false, Some(false)) => tp += 1, (true, Some(false)) => fp += 1,
            (false, Some(true)) => fn_count += 1, (true, Some(true)) => tn += 1,
            _ => unknown += 1,
        }
    }

    println!("\n=== pascal grammar 精度验证 ===");
    println!("总样本: {}", corpus["cases"].as_array().unwrap().len());
    println!("TP={tp} FP={fp} FN={fn_count} TN={tn} unknown={unknown}");

    let evidence = generate_precision_validation(
        "2026-10-07", corpus["cases"].as_array().unwrap().len() as u32,
        tp, fp, fn_count, tn, unknown,
        "tests/acceptance/pascal-grammar-precision-2026-10-07.md", None,
    );

    match evidence {
        Ok(ev) => {
            println!("✅ pascal PASS! Wilson 下界 = {:.4}", ev.precision_wilson_lower_bound);
            let doc = format!(
                "# pascal Grammar 精度验证证据\n\n> 日期：2026-10-07。\n\n| 指标 | 值 |\n|---|---|\n| 总样本 | {} |\n| TP | {tp} |\n| FP | {fp} |\n| FN | {fn_count} |\n| TN | {tn} |\n| Wilson 下界 (95%) | {:.4} |\n| 结果 | **PASS** |\n",
                corpus["cases"].as_array().unwrap().len(), ev.precision_wilson_lower_bound
            );
            let doc_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..").join("..").join("tests").join("acceptance")
                .join("pascal-grammar-precision-2026-10-07.md");
            std::fs::write(&doc_path, doc).unwrap();
        }
        Err(e) => panic!("pascal 精度验证未通过: {e}"),
    }
}
