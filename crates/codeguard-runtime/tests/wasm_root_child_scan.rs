#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::{WasmGrammar, scan_wasm_root_child};

fn go() -> WasmGrammar {
    WasmGrammar::load(
        "go",
        include_bytes!("../../../grammars/go/parser.wasm"),
        "4eda5d91c99ca981e88bc7d3d33f0db166b4bab0a84d0021a9abf39b364c78ef",
        14,
    )
    .unwrap()
}

#[test]
fn absent_go_package_is_a_root_fact_even_without_parser_errors() {
    let mut grammar = go();
    for source in [
        "x := 1\n",
        "func f() {}\n",
        "// package main\nfunc f() {}\n",
        "var message = `package main`\n",
        "/* package main */\n",
        "",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error(), "{source:?}");
        let scan = scan_wasm_root_child(&tree, "package_clause", 128).unwrap();
        assert_eq!(scan.root_syntax_kind, "source_file");
        assert_eq!(scan.child_syntax_kind, "package_clause");
        assert!(!scan.present, "{source:?}");
        assert!(!scan.truncated);
    }
}

#[test]
fn valid_go_declarations_survive_comments_unicode_and_line_endings() {
    let mut grammar = go();
    for source in [
        "package main\n",
        "//go:build linux\n\npackage main\nfunc f() {}\n",
        "/* package fake */\npackage real\nvar message = `package fake`\n",
        "package 名字\r\nfunc f() {}\r\n",
        "// comment\r\npackage main\r\n",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error(), "{source:?}");
        let scan = scan_wasm_root_child(&tree, "package_clause", 128).unwrap();
        assert!(scan.present, "{source:?}");
        assert!(!scan.truncated);
    }
}

#[test]
fn root_budget_does_not_prove_absence_or_completeness() {
    let mut grammar = go();
    let tree = grammar.parse(b"// one\n// two\npackage main\n").unwrap();
    let scan = scan_wasm_root_child(&tree, "package_clause", 1).unwrap();
    assert!(!scan.present);
    assert!(scan.truncated);
    let scan = scan_wasm_root_child(&tree, "package_clause", 128).unwrap();
    assert!(scan.present);
    assert!(!scan.truncated);
    let tree = grammar.parse(b"package main\nfunc f() {}\n").unwrap();
    let scan = scan_wasm_root_child(&tree, "package_clause", 1).unwrap();
    assert!(scan.present);
    assert!(scan.truncated);
    assert!(scan_wasm_root_child(&tree, "package_clause", 0).is_err());
    assert!(scan_wasm_root_child(&tree, "package_clause", 200_001).is_err());
    assert!(scan_wasm_root_child(&tree, "", 128).is_err());
}

#[test]
fn root_facts_are_language_independent_and_not_source_verdicts() {
    let mut grammar = WasmGrammar::load(
        "rust",
        include_bytes!("../../../grammars/rust/parser.wasm"),
        "206031e0f67fb41ecae505868ca3bb917df7375031aebafd2f97314a849713fe",
        15,
    )
    .unwrap();
    let tree = grammar.parse(b"fn f() {}\n").unwrap();
    let scan = scan_wasm_root_child(&tree, "package_clause", 128).unwrap();
    assert_eq!(scan.root_syntax_kind, "source_file");
    assert!(!scan.present);
    assert!(!scan.truncated);
    // 即使根节点类型相同，也不能据缺package事实对合法Rust源码作Go判断。
    assert!(!tree.root_node().has_error());
}
