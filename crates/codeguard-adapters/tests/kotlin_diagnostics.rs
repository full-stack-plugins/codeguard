use codeguard_adapters::parse_kotlin_diagnostics;

#[test]
fn syntax_and_context_remain_distinct_and_duplicate_locations_merge() {
    let report=b"/f.kt:1:10: error: [SYNTAX] Type expected.\nfun f(x: ) = x\n         ^\n/f.kt:1:10: error: [SYNTAX] Incomplete code.\nfun f(x: ) = x\n         ^\n/f.kt:1:13: error: [UNRESOLVED_REFERENCE] Unknown.\nfun f(x: ) = x\n            ^\n";
    let parsed = parse_kotlin_diagnostics(report, "/f.kt", "fun f(x: ) = x\n").unwrap();
    assert_eq!(parsed.syntax.len(), 1);
    assert_eq!(parsed.context.len(), 1);
    assert_eq!(parsed.syntax[0].rule_id, "kotlin.syntax");
    assert_eq!(
        parsed.context[0].rule_id,
        "kotlin.context.UNRESOLVED_REFERENCE"
    );
}

#[test]
fn utf16_locations_map_to_original_utf8_and_reject_surrogate_interiors() {
    let source = "val s = \"😀\"; val 文 = ;\n";
    let prefix = "val s = \"😀\"; val 文 = ";
    let column = prefix.encode_utf16().count() + 1;
    let report = format!(
        "/f.kt:1:{column}: error: [SYNTAX] Expression expected.\n{source}{}^\n",
        " ".repeat(column - 1)
    );
    let parsed = parse_kotlin_diagnostics(report.as_bytes(), "/f.kt", source).unwrap();
    assert_eq!(parsed.syntax[0].column_byte, prefix.len() + 1);
    assert_eq!(parsed.syntax[0].column_utf16, column);
    assert!(
        parse_kotlin_diagnostics(b"/f.kt:1:2: error: [SYNTAX] Bad.\n", "/f.kt", "😀\n").is_none()
    );
}

#[test]
fn foreign_paths_unknown_rows_bad_codes_and_out_of_range_positions_are_rejected() {
    for report in [
        "/other.kt:1:1: error: [SYNTAX] Bad.\n",
        "/f.kt:0:1: error: [SYNTAX] Bad.\n",
        "/f.kt:1:0: error: [SYNTAX] Bad.\n",
        "/f.kt:1:99: error: [SYNTAX] Bad.\n",
        "/f.kt:1:1: error: [syntax] Bad.\n",
        "No space left on device\n",
        "/f.kt:1:1: error: [SYNTAX] Bad.\nchanged source\n",
    ] {
        assert!(
            parse_kotlin_diagnostics(report.as_bytes(), "/f.kt", "x\n").is_none(),
            "{report}"
        );
    }
}

#[test]
fn empty_output_is_not_itself_a_success_and_diagnostic_budget_is_bounded() {
    assert!(
        parse_kotlin_diagnostics(b"", "/f.kt", "x\n")
            .unwrap()
            .syntax
            .is_empty()
    );
    let mut report = String::new();
    let source = "x\n".repeat(33);
    for line in 1..=33 {
        report += &format!("/f.kt:{line}:1: error: [SYNTAX] Bad.\n");
    }
    assert!(parse_kotlin_diagnostics(report.as_bytes(), "/f.kt", &source).is_none());
}
