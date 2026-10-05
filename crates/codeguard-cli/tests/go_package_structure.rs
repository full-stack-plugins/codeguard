#![cfg(all(feature = "wasm-precheck", unix))]

#[path = "../src/go_syntax_probe.rs"]
mod go_syntax_probe;

use codeguard_adapters::{go_package_rule_sha256, missing_go_package_candidate};
use codeguard_runtime::{WasmGrammar, scan_wasm_recoveries, scan_wasm_root_child};
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn go() -> WasmGrammar {
    WasmGrammar::load(
        "go",
        include_bytes!("../../../grammars/go/parser.wasm"),
        "4eda5d91c99ca981e88bc7d3d33f0db166b4bab0a84d0021a9abf39b364c78ef",
        14,
    )
    .unwrap()
}

fn cases() -> Vec<Value> {
    let corpus: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/go-native-grammar-input-2026-10-05.json"
    ))
    .unwrap();
    corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["language"] == "go")
        .cloned()
        .collect()
}

fn classifications(grammar: &mut WasmGrammar, source: &[u8]) -> (bool, bool) {
    let tree = grammar.parse(source).unwrap();
    let raw = scan_wasm_recoveries(&tree, 128).unwrap();
    let package = scan_wasm_root_child(&tree, "package_clause", 200_000).unwrap();
    assert!(!raw.truncated);
    let candidate = missing_go_package_candidate(
        "go",
        true,
        &package.root_syntax_kind,
        &package.child_syntax_kind,
        package.present,
        package.truncated,
    )
    .expect("complete fixed grammar root facts");
    (
        raw.recoveries.is_empty(),
        raw.recoveries.is_empty() && !candidate,
    )
}

#[test]
fn go_whole_file_structure_catches_missing_packages_without_relabeling_raw_recoveries() {
    let mut grammar = go();
    let cases = cases();
    assert_eq!(cases.len(), 20);
    let mut raw_false_negatives = Vec::new();
    for case in cases {
        let expected = case["expected_valid"].as_bool().unwrap();
        let (raw_valid, combined_valid) =
            classifications(&mut grammar, case["source"].as_str().unwrap().as_bytes());
        if raw_valid && !expected {
            raw_false_negatives.push(case["id"].as_str().unwrap().to_owned());
        }
        assert_eq!(combined_valid, expected, "{}", case["id"]);
    }
    assert_eq!(
        raw_false_negatives,
        [
            "go-missing_package_statement",
            "go-missing_package_function"
        ]
    );
}

#[test]
#[ignore = "requires explicitly selected existing Go 1.23.4 via CODEGUARD_GO_BIN"]
fn pinned_go_structure_matches_fresh_native_whole_file_observations() {
    let tool = std::env::var("CODEGUARD_GO_BIN").expect("select existing pinned Go SDK");
    let tool = Path::new(&tool);
    let binding = go_syntax_probe::companion_identity(tool).unwrap();
    let cancelled = AtomicBool::new(false);
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut grammar = go();
    let mut native_tool_sha = None;
    let mut counts = [0usize; 5]; // TP、FP、FN、TN、unknown；原始层另行保留。
    for case in cases() {
        let source = case["source"].as_str().unwrap().as_bytes();
        let native = go_syntax_probe::observe(tool, source, deadline, &cancelled);
        let (_, combined_valid) = classifications(&mut grammar, source);
        if !matches!(
            native["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) {
            assert_eq!(
                case["id"], "go-logical_line_positions_unresolved",
                "{native}"
            );
            assert_eq!(native["reason"], "go_syntax_logical_positions_unresolved");
            counts[4] += 1;
            continue;
        }
        assert_eq!(native["companion_binding_sha256"], binding);
        let observed_sha = native["tool_sha256"].as_str().unwrap();
        if let Some(expected_sha) = &native_tool_sha {
            assert_eq!(observed_sha, expected_sha);
        } else {
            native_tool_sha = Some(observed_sha.to_owned());
        }
        let native_valid = native["diagnostics"].as_array().unwrap().is_empty();
        assert_eq!(combined_valid, native_valid, "{}: {native}", case["id"]);
        counts[match (native_valid, combined_valid) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        }] += 1;
    }
    assert_eq!(counts, [7, 0, 0, 12, 1]);
    // 专门验证新规则的误报反例，不将这些开发回归称为独立holdout。
    let controls = [
        ("x := 1\n", false),
        ("func f() {}\n", false),
        ("// package main\nfunc f() {}\n", false),
        ("var message = `package main`\n", false),
        ("/* package main */\n", false),
        ("", false),
        ("package main\n", true),
        ("//go:build linux\n\npackage main\nfunc f() {}\n", true),
        (
            "/* package fake */\npackage real\nvar message = `package fake`\n",
            true,
        ),
        ("package 名字\r\nfunc f() {}\r\n", true),
        ("// comment\r\npackage main\r\n", true),
    ];
    for (source, expected_valid) in controls {
        let native = go_syntax_probe::observe(tool, source.as_bytes(), deadline, &cancelled);
        assert!(
            matches!(
                native["status"].as_str(),
                Some("completed" | "diagnostics_observed")
            ),
            "{source:?}: {native}"
        );
        assert_eq!(native["tool_sha256"], native_tool_sha.as_deref().unwrap());
        assert_eq!(native["companion_binding_sha256"], binding);
        let native_valid = native["diagnostics"].as_array().unwrap().is_empty();
        let (_, combined_valid) = classifications(&mut grammar, source.as_bytes());
        assert_eq!(native_valid, expected_valid, "{source:?}: {native}");
        assert_eq!(combined_valid, native_valid, "{source:?}: {native}");
    }
    assert_eq!(go_syntax_probe::companion_identity(tool).unwrap(), binding);
    println!(
        "Go package structure development-only: TP/FP/FN/TN/unknown={counts:?}; rule_sha256={}; tool_sha256={}; companion_binding_sha256={binding}; public_route_integrated=false; grammar_qualified=false",
        go_package_rule_sha256(),
        native_tool_sha.unwrap()
    );
}
