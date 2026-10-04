use codeguard_adapters::parse_grammar_test_corpus;

#[test]
fn parser_preserves_source_bytes_and_last_expectation() {
    let text = "====\r\nClean\r\n====\r\nx = 1\r\n---\r\n(root (identifier))\r\n====\r\nBroken\r\n====\r\nx =\r\n---\r\n(root\r\n (MISSING expression))";
    let cases = parse_grammar_test_corpus(text).unwrap();
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].source, "x = 1\r\n");
    assert!(!cases[0].expected_error);
    assert_eq!(cases[1].source, "x =\r\n");
    assert!(cases[1].expected_error);
}

#[test]
fn recovery_names_inside_quotes_or_longer_symbols_do_not_create_error_labels() {
    for expected in [
        r#"(root "(ERROR)" "(MISSING)")"#,
        "(root (ERROR_LIKE) (MISSING_VALUE))",
    ] {
        let text = format!("====\nLabel\n====\nx\n---\n{expected}");
        assert!(!parse_grammar_test_corpus(&text).unwrap()[0].expected_error);
    }
}

#[test]
fn malformed_expectations_missing_separator_and_directives_are_rejected() {
    for text in [
        "",
        "====\nA\n====\nx",
        "====\nA\n====\nx\n---\n",
        "====\nA\n====\nx\n---\n(root (ERROR)",
        "====\nA\n====\nx\n---\n(root \"unterminated)",
        "====\nA\n:skip\n====\nx\n---\n(root)",
        "====\nA\n====\nx\n---\n()",
        "====\nA\n====\nx\n---\n(root ())",
        "====\nA\n====\nx\n====\nB\n====\ny\n---\n(root)",
    ] {
        assert!(parse_grammar_test_corpus(text).is_err(), "{text}");
    }
}

#[test]
fn all_existing_dart_corpora_import_exactly_150_cases_with_four_negative_labels() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars/dart/corpus");
    let mut total = 0;
    let mut negative = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }
        let samples = parse_grammar_test_corpus(&std::fs::read_to_string(p).unwrap()).unwrap();
        total += samples.len();
        negative += samples.iter().filter(|c| c.expected_error).count();
    }
    assert_eq!(total, 150);
    assert_eq!(negative, 4);
}
