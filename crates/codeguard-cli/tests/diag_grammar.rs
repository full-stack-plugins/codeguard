//! Grammar 诊断测试：查看 grammar 实际解析行为。

#![cfg(feature = "wasm-precheck")]

use sha2::Digest;
use std::path::Path;

#[test]
fn diag_solidity_grammar() {
    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("grammars")
        .join("solidity")
        .join("parser.wasm");
    let wasm = std::fs::read(&wasm_path).unwrap();
    let sha = format!("{:x}", sha2::Sha256::digest(&wasm));
    let mut g = codeguard_runtime::WasmGrammar::load("solidity", &wasm, &sha, 14).unwrap();

    let samples = [
        ("empty", ""),
        ("contract_empty", "contract C {}"),
        ("contract_pragma", "pragma solidity ^0.8.0;\ncontract C {}"),
        ("contract_uint", "contract C { uint x; }"),
        ("contract_func", "contract C { function f() public {} }"),
    ];

    for (name, src) in samples {
        match g.parse(src.as_bytes()) {
            Ok(tree) => {
                let mut errors = Vec::new();
                let mut cursor = tree.walk();
                loop {
                    let node = cursor.node();
                    if node.kind() == "ERROR" || node.is_missing() {
                        errors.push(format!(
                            "{}@{}-{}",
                            node.kind(),
                            node.start_byte(),
                            node.end_byte()
                        ));
                    }
                    if !cursor.goto_next_sibling() && !cursor.goto_first_child() {
                        break;
                    }
                }
                if errors.is_empty() {
                    println!("{}: VALID", name);
                } else {
                    println!("{}: INVALID errors={:?}", name, errors);
                }
            }
            Err(e) => println!("{}: PARSE FAILED: {}", name, e),
        }
    }
}
