#![cfg(feature = "wasm-precheck")]
use codeguard_runtime::{WasmGrammar, scan_wasm_form_terminators};
fn grammar() -> WasmGrammar {
    WasmGrammar::load(
        "erlang",
        include_bytes!("../../../grammars/erlang/parser.wasm"),
        "dbab33f03e07b89f4385fcdd48d87d86ba35c82a0a426788d55b8c25410bc491",
        14,
    )
    .unwrap()
}
#[test]
fn budget_never_turns_unvisited_continuation_into_final_semicolon() {
    let mut grammar = grammar();
    let tree = grammar
        .parse(b"f(0) -> zero;\n% comment .\nf(_) -> other.\n")
        .unwrap();
    let (facts, truncated) = scan_wasm_form_terminators(&tree, "fun_decl", 128, 3).unwrap();
    assert!(facts.is_empty());
    assert!(truncated);
    let (facts, truncated) = scan_wasm_form_terminators(&tree, "fun_decl", 128, 200_000).unwrap();
    assert!(facts.is_empty());
    assert!(!truncated);
    assert!(scan_wasm_form_terminators(&tree, "fun_decl", 0, 200_000).is_err());
    assert!(scan_wasm_form_terminators(&tree, "fun_decl", 128, 0).is_err());
}
#[test]
fn eof_and_other_forms_resolve_pending_semicolon_but_result_budget_stays_incomplete() {
    let mut grammar = grammar();
    for source in [
        b"f() -> ok; % trailing .".as_slice(),
        b"f() -> ok;\n-export([f/0]).",
    ] {
        let tree = grammar.parse(source).unwrap();
        let (facts, truncated) =
            scan_wasm_form_terminators(&tree, "fun_decl", 128, 200_000).unwrap();
        assert_eq!(facts.len(), 1);
        assert!(!truncated);
        assert_eq!(&source[facts[0].start_byte..facts[0].end_byte], b";");
    }
    let tree = grammar.parse(b"f()->ok\ng()->ok\nh()->ok\n").unwrap();
    let (facts, truncated) = scan_wasm_form_terminators(&tree, "fun_decl", 1, 200_000).unwrap();
    assert_eq!(facts.len(), 1);
    assert!(truncated);
    assert_eq!(facts[0].start_byte, facts[0].end_byte);
}
