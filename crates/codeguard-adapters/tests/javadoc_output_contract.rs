use codeguard_adapters::{JavadocParseState, parse_detailed_javadoc_output, parse_javadoc_output};

#[test]
fn jdk21_missing_comment_param_and_return_are_structured() {
    let path = "/private/Bad.java";
    let output = b"/private/Bad.java:2: warning: no comment\npublic class Bad {}\n       ^\n/private/Bad.java:3: warning: no @param for value\n  public int run(int value) { return value; }\n             ^\n/private/Bad.java:3: warning: no @return\n  public int run(int value) { return value; }\n             ^\n3 warnings\n";
    let source = b"\npublic class Bad {}\n  public int run(int value) { return value; }\n";
    let parsed = parse_javadoc_output(output, path, source);
    assert_eq!(parsed.state, JavadocParseState::ValidDiagnostics);
    assert_eq!(parsed.diagnostics.len(), 3);
    assert_eq!(parsed.diagnostics[0].rule_id, "JavadocMissingComment");
    assert_eq!(parsed.diagnostics[0].line, 2);
    assert_eq!(parsed.diagnostics[0].column, 8);
    assert_eq!(parsed.diagnostics[1].rule_id, "JavadocMissingParam");
    assert_eq!(parsed.diagnostics[2].rule_id, "JavadocMissingReturn");
}

#[test]
fn unknown_or_out_of_scope_output_never_becomes_a_clean_result() {
    for output in [
        "/other/Bad.java:1: warning: no comment\nclass Bad {}\n^\n1 warning\n",
        "/private/Bad.java:1: warning: unknown diagnostic\nclass Bad {}\n^\n1 warning\n",
        "/private/Bad.java:1: warning: no comment\nclass Bad {}\n^\n2 warnings\n",
        "/private/Bad.java:1: error: symbol not found\nclass Bad {}\n^\n1 error\n",
    ] {
        let parsed =
            parse_javadoc_output(output.as_bytes(), "/private/Bad.java", b"class Bad {}\n");
        assert_eq!(parsed.state, JavadocParseState::Incomplete, "{output}");
    }
}

#[test]
fn diagnostic_source_excerpt_must_match_the_scanned_bytes() {
    let output =
        b"/private/Bad.java:1: warning: no comment\npublic class Bad {}\n       ^\n1 warning\n";
    let parsed = parse_javadoc_output(output, "/private/Bad.java", b"public class Good {}\n");
    assert_eq!(parsed.state, JavadocParseState::Incomplete);
    assert_eq!(parsed.reason, Some("javadoc_source_line_mismatch"));
}

#[test]
fn detailed_descriptions_have_distinct_source_bound_rules() {
    let source = b"public class Bad {}\n";
    for (message, rule) in [
        ("empty comment", "JavadocEmptyComment"),
        ("no main description", "JavadocMissingMainDescription"),
        ("no description for @param", "JavadocEmptyParamDescription"),
        (
            "no description for @return",
            "JavadocEmptyReturnDescription",
        ),
        (
            "no description for @throws",
            "JavadocEmptyThrowsDescription",
        ),
    ] {
        let output = format!(
            "/private/Bad.java:1: warning: {message}\npublic class Bad {{}}\n       ^\n1 warning\n"
        );
        let legacy = parse_javadoc_output(output.as_bytes(), "/private/Bad.java", source);
        assert_eq!(legacy.state, JavadocParseState::Incomplete);
        let parsed = parse_detailed_javadoc_output(output.as_bytes(), "/private/Bad.java", source);
        assert_eq!(
            parsed.state,
            JavadocParseState::ValidDiagnostics,
            "{message}"
        );
        assert_eq!(parsed.diagnostics[0].rule_id, rule);
    }
}
