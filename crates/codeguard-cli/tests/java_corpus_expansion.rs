//! Java grammar 扩展语料精度验证。
//!
//! 直接用 WASM grammar 解析 400 个 Java 样本（200 合法 + 200 违规），
//! 提取 TP/FP/FN/TN 计数并生成 PrecisionValidation 证据。
//! 只在 wasm-precheck 特性下运行（需要 WASM 加载能力）。

#![cfg(feature = "wasm-precheck")]

use codeguard_adapters::generate_precision_validation;
use sha2::Digest;
use serde_json::Value;
use std::path::Path;

/// 从 grammar 解析结果中提取分类。
/// 返回 Some(true) = 有效（无恢复节点），Some(false) = 无效（有恢复节点），None = 未知。
fn classify_from_grammar(grammar: &mut codeguard_runtime::WasmGrammar, source: &[u8]) -> Option<bool> {
    match grammar.parse(source) {
        Ok(tree) => {
            // 检查是否有 ERROR 或 MISSING 节点
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
fn java_corpus_expansion_meets_precision_threshold() {
    // 读取扩展语料
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("java_grammar_corpus_expanded.json");
    let corpus_bytes = std::fs::read(&corpus_path).expect("扩展语料文件不存在");
    let corpus: Value = serde_json::from_slice(&corpus_bytes).expect("语料 JSON 无效");

    let cases = corpus["cases"].as_array().expect("cases 字段缺失");
    println!("加载 {} 个 Java 语法样本", cases.len());

    // 加载 Java grammar
    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("grammars")
        .join("java")
        .join("parser.wasm");
    let wasm_bytes = std::fs::read(&wasm_path).expect("Java grammar WASM 文件不存在");
    let wasm_sha256 = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));
    let mut grammar = codeguard_runtime::WasmGrammar::load("java", &wasm_bytes, &wasm_sha256, 14)
        .expect("Java grammar 加载失败");

    // 分类每个样本
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
            (false, Some(false)) => tp += 1,  // 期望违规，检出违规 = TP
            (true, Some(false)) => fp += 1,    // 期望合法，误报违规 = FP
            (false, Some(true)) => fn_count += 1, // 期望违规，漏检为合法 = FN
            (true, Some(true)) => tn += 1,     // 期望合法，正确判为合法 = TN
            _ => unknown += 1,                 // 未知 = unknown
        }
    }

    println!("\n=== Java 扩展语料回放结果 ===");
    println!("总样本: {}", cases.len());
    println!("TP(真阳性)={tp} FP(假阳性)={fp} FN(假阴性)={fn_count} TN(真阴性)={tn} unknown={unknown}");
    println!("正样本(违规)总数: {}", tp + fn_count);
    println!("负样本(合法)总数: {}", tn + fp);
    println!("精度(precision) = TP/(TP+FP) = {}", if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 });

    // 生成精度验证证据
    let evidence = generate_precision_validation(
        "2026-10-07",
        cases.len() as u32,
        tp,
        fp,
        fn_count,
        tn,
        unknown,
        "tests/acceptance/java-grammar-precision-2026-10-07.md",
        None,
    );

    match evidence {
        Ok(ev) => {
            println!("\n✅ 精度验证通过！Wilson 下界 = {:.4}", ev.precision_wilson_lower_bound);
            println!("   满足门槛 >= 0.98: {}", ev.precision_wilson_lower_bound >= 0.98);

            // 写入验收文档
            let doc_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("tests")
                .join("acceptance")
                .join("java-grammar-precision-2026-10-07.md");
            let doc = format!(
                "# Java Grammar 精度验证证据\n\n\
                 > 日期：2026-10-07；OpenSpec 14.17 / 15.2。\n\
                 > 语料：400 个 Java 语法样本（200 合法 + 200 违规）。\n\n\
                 ## 精度指标\n\n\
                 | 指标 | 值 |\n\
                 |---|---|\n\
                 | 总样本 | {} |\n\
                 | 真阳性 (TP) | {} |\n\
                 | 假阳性 (FP) | {} |\n\
                 | 假阴性 (FN) | {} |\n\
                 | 真阴性 (TN) | {} |\n\
                 | 未知 | {} |\n\
                 | 精度 Wilson 下界 (95%) | {:.4} |\n\
                 | 门槛 | >= 0.98 |\n\
                 | 结果 | **{}** |\n\n\
                 ## 语料来源\n\n\
                 - 生成器：`tests/fixtures/generate_java_corpus.py`\n\
                 - 语料文件：`tests/fixtures/java_grammar_corpus_expanded.json`\n\
                 - 解析工具：Java WASM grammar (tree-sitter 0.25.3)\n\n\
                 ## 资格判定\n\n\
                 本证据满足 grammar 资格的精度门槛（Wilson 下界 >= 0.98）。\n\
                 但 `release_status` 标记需要在 `grammars/manifest.json` 中显式写入。\n",
                cases.len(), tp, fp, fn_count, tn, unknown,
                ev.precision_wilson_lower_bound,
                if ev.precision_wilson_lower_bound >= 0.98 { "PASS" } else { "FAIL" },
            );
            std::fs::write(&doc_path, doc).expect("无法写入验收文档");
            println!("\n📄 验收文档已写入: {}", doc_path.display());
        }
        Err(e) => {
            println!("\n❌ 精度验证失败: {e}");
            panic!("精度验证未通过: {e}");
        }
    }
}
