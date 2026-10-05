#![cfg(feature = "wasm-precheck")]
use codeguard_runtime::{WasmGrammar, scan_wasm_empty_blocks};

fn python() -> WasmGrammar {
    WasmGrammar::load(
        "python",
        include_bytes!("../../../grammars/python/parser.wasm"),
        "a7fdc587e77bd729b9f5b783c659be23c896e305a2c374472bed7114d9e01fac",
        14,
    )
    .unwrap()
}

#[test]
fn python_empty_suite_facts_exist_without_parser_recovery() {
    let mut grammar = python();
    for (source, owner) in [
        ("def run():\n", "function_definition"),
        ("if True:\npass\n", "if_statement"),
        ("def run():\n    # comment only\n", "function_definition"),
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        let scan = scan_wasm_empty_blocks(&tree, 32).unwrap();
        assert!(!scan.truncated);
        assert!(
            scan.blocks
                .iter()
                .any(|block| block.parent_syntax_kind == owner),
            "{} {:?}",
            tree.root_node().to_sexp(),
            scan
        );
        assert!(
            scan.blocks
                .iter()
                .all(|block| block.start_byte <= block.end_byte && block.end_byte <= source.len())
        );
    }
}

#[test]
fn valid_python_suites_and_non_suite_source_are_not_empty() {
    let mut grammar = python();
    for source in [
        "def run(): pass\n",
        "def run():\n    ...\n",
        "def run():\n    # comment\n    pass\n",
        "if True:\n    pass\n",
        "class C:\n    \"\"\"docstring\"\"\"\n",
        "# comments only\n",
        "\n",
        "def run():\r\n    名字 = 1\r\n",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        let scan = scan_wasm_empty_blocks(&tree, 32).unwrap();
        assert!(!scan.truncated);
        assert!(scan.blocks.is_empty(), "{source:?}: {:?}", scan);
    }
}

#[test]
fn empty_block_budget_retains_incomplete_and_other_languages_are_only_facts() {
    let mut grammar = python();
    let tree = grammar.parse(b"def first():\ndef second():\n").unwrap();
    let scan = scan_wasm_empty_blocks(&tree, 1).unwrap();
    assert_eq!(scan.blocks.len(), 1);
    assert!(scan.truncated);
    assert!(scan_wasm_empty_blocks(&tree, 0).is_err());
    assert!(scan_wasm_empty_blocks(&tree, 1025).is_err());
    let mut rust = WasmGrammar::load(
        "rust",
        include_bytes!("../../../grammars/rust/parser.wasm"),
        "206031e0f67fb41ecae505868ca3bb917df7375031aebafd2f97314a849713fe",
        15,
    )
    .unwrap();
    let tree = rust.parse(b"fn run() {}\n").unwrap();
    assert!(!tree.root_node().has_error());
    let scan = scan_wasm_empty_blocks(&tree, 32).unwrap();
    assert!(!scan.truncated);
    // Rust 的合法空函数体也产生事实，证明该事实不能直接当成语言违规。
    assert_eq!(scan.blocks.len(), 1);
    assert!(
        scan.blocks
            .iter()
            .all(|block| block.parent_syntax_kind == "function_item")
    );
}

#[test]
fn whole_tree_visit_budget_cannot_look_complete_with_zero_facts() {
    let mut grammar = python();
    let source = "pass\n".repeat(120_000);
    let tree = grammar.parse(source.as_bytes()).unwrap();
    let scan = scan_wasm_empty_blocks(&tree, 32).unwrap();
    assert!(scan.blocks.is_empty());
    assert!(
        scan.truncated,
        "a zero-fact traversal must retain the exhausted node budget"
    );
}

#[test]
fn structural_child_inspections_consume_budget_and_preserve_existing_facts() {
    let mut grammar = python();
    let tail = "pass\n".repeat(60_000);
    for (prefix, expected_blocks) in [("", 0), ("def missing():\n", 1)] {
        let source = format!("{prefix}{tail}");
        let tree = grammar.parse(source.as_bytes()).unwrap();
        let scan = scan_wasm_empty_blocks(&tree, 32).unwrap();
        assert_eq!(scan.blocks.len(), expected_blocks);
        assert!(
            scan.truncated,
            "child inspection work must consume the structural traversal budget"
        );
    }
}
