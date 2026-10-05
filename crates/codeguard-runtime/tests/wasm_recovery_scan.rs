#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::{WasmGrammar, scan_wasm_recoveries};

fn java_grammar() -> WasmGrammar {
    WasmGrammar::load(
        "java",
        include_bytes!("../../../grammars/java/parser.wasm"),
        "181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077",
        14,
    )
    .expect("固定 Java grammar")
}

#[test]
fn hidden_swift_missing_type_is_incomplete_even_with_zero_records() {
    let mut grammar = WasmGrammar::load(
        "swift",
        include_bytes!("../../../grammars/swift/parser.wasm"),
        "cc77a63b8487956270e2f385e29a03ba0773ba532a3c8a8844a26b4c98793843",
        15,
    )
    .unwrap();
    let tree = grammar.parse(b"func f(_ x: ) {}\n").unwrap();
    assert!(tree.root_node().has_error());
    let scan = scan_wasm_recoveries(&tree, 128).unwrap();
    assert!(scan.recoveries.is_empty());
    assert!(
        scan.truncated,
        "a hidden token cannot be classified as valid syntax"
    );
    let tree = grammar.parse(b"func f(_ x: Int) {}\n").unwrap();
    let scan = scan_wasm_recoveries(&tree, 128).unwrap();
    assert!(scan.recoveries.is_empty());
    assert!(!scan.truncated);
}

#[test]
fn hidden_kotlin_missing_tokens_do_not_look_clean() {
    let mut grammar = WasmGrammar::load(
        "kotlin",
        include_bytes!("../../../grammars/kotlin/parser.wasm"),
        "c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48",
        14,
    )
    .unwrap();
    for source in ["fun f(x: ) = x\n", "object C { val value = 1 }\n"] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(tree.root_node().has_error());
        let scan = scan_wasm_recoveries(&tree, 128).unwrap();
        assert!(
            scan.truncated,
            "hidden grammar error must remain incomplete"
        );
    }
    let clean = grammar.parse(b"class C {}\n").unwrap();
    let scan = scan_wasm_recoveries(&clean, 128).unwrap();
    assert!(scan.recoveries.is_empty());
    assert!(!scan.truncated);
}

#[test]
fn error_and_missing_recoveries_keep_byte_positions() {
    let mut grammar = java_grammar();
    let clean = grammar
        .parse("class A { String s = \"中\"; }".as_bytes())
        .unwrap();
    let clean_scan = scan_wasm_recoveries(&clean, 32).unwrap();
    assert!(clean_scan.recoveries.is_empty());
    assert!(!clean_scan.truncated);

    let missing_source = "class A { String s = \"中\";";
    let missing = grammar.parse(missing_source.as_bytes()).unwrap();
    let missing_scan = scan_wasm_recoveries(&missing, 32).unwrap();
    assert!(
        missing_scan
            .recoveries
            .iter()
            .any(|node| node.kind == "MISSING")
    );
    assert!(missing_scan.recoveries.iter().all(
        |node| node.start_byte <= missing_source.len() && node.end_byte <= missing_source.len()
    ));
    assert!(missing_scan.recoveries.iter().any(|node| {
        node.kind == "MISSING"
            && node.start_byte == missing_source.len()
            && node.start_column_byte == missing_source.len()
    }));

    let multiline_source = "class A {\r\n  String s = \"中\";\r\n";
    let multiline = grammar.parse(multiline_source.as_bytes()).unwrap();
    let multiline_scan = scan_wasm_recoveries(&multiline, 32).unwrap();
    let anchor = multiline_source.find(";\r\n").unwrap() + 1;
    let second_line_start = multiline_source.find('\n').unwrap() + 1;
    assert!(
        multiline_scan.recoveries.iter().any(|node| {
            node.kind == "MISSING"
                && node.start_byte == anchor
                && node.start_row == 1
                && node.start_column_byte == anchor - second_line_start
        }),
        "{:?}",
        multiline_scan.recoveries
    );

    let invalid = grammar.parse(b"class A { int x = ; }").unwrap();
    let invalid_scan = scan_wasm_recoveries(&invalid, 32).unwrap();
    assert!(
        invalid_scan
            .recoveries
            .iter()
            .any(|node| node.kind == "ERROR")
    );
}

#[test]
fn diagnostic_budget_is_explicitly_incomplete() {
    let mut grammar = java_grammar();
    let invalid = grammar.parse(b"class A { int x = ; int y = ; }").unwrap();
    let full = scan_wasm_recoveries(&invalid, 32).unwrap();
    let groups = full
        .recoveries
        .iter()
        .filter(|node| node.kind == "ERROR")
        .map(|node| node.group_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(groups.len(), 2, "并列错误不能合并成一个修复任务");
    let limited = scan_wasm_recoveries(&invalid, 1).unwrap();
    assert_eq!(limited.recoveries.len(), 1);
    assert!(limited.truncated);
    assert!(scan_wasm_recoveries(&invalid, 0).is_err());
}

#[test]
fn typescript_recoveries_are_observations_with_original_byte_ranges() {
    let mut grammar = WasmGrammar::load(
        "typescript",
        include_bytes!("../../../grammars/typescript/parser.wasm"),
        "3a44d634c9840dccec36f33b99592bd086a2f940e9cd80c64347c45b7dce662f",
        14,
    )
    .unwrap();
    let source = b"const value: number = ;";
    let tree = grammar.parse(source).unwrap();
    let scan = scan_wasm_recoveries(&tree, 32).unwrap();
    assert!(!scan.recoveries.is_empty());
    assert!(!scan.truncated);
    assert!(scan.recoveries.iter().all(|node| {
        matches!(node.kind, "ERROR" | "MISSING")
            && node.start_byte <= node.end_byte
            && node.end_byte <= source.len()
    }));

    let separate = grammar
        .parse(b"const a: number = ;\nconst b: number = ;")
        .unwrap();
    let separate_scan = scan_wasm_recoveries(&separate, 32).unwrap();
    let groups = separate_scan
        .recoveries
        .iter()
        .map(|node| node.group_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(groups.len(), 2, "两行独立错误不能被位置邻近规则合并");
}
