use codeguard_adapters::checkstyle_native_failure_reason;

const FAILURE: &str = "com.puppycrawl.tools.checkstyle.api.CheckstyleException: cannot initialize module JavadocType\nCaused by: com.puppycrawl.tools.checkstyle.api.CheckstyleException: illegal value '[' for property 'authorFormat'\nCaused by: java.util.regex.PatternSyntaxException: Unclosed character class\n";

#[test]
fn fixed_native_configuration_regex_failure_is_not_a_source_violation() {
    assert_eq!(
        checkstyle_native_failure_reason(Some(254), FAILURE.as_bytes()),
        Some("checkstyle_configuration_regex_invalid")
    );
    for exit in [None, Some(0), Some(1), Some(2)] {
        assert_eq!(
            checkstyle_native_failure_reason(exit, FAILURE.as_bytes()),
            None
        );
    }
    for text in [
        FAILURE.replace("authorFormat", "unrecognizedProperty"),
        FAILURE.replace(
            "java.util.regex.PatternSyntaxException",
            "java.lang.RuntimeException",
        ),
        FAILURE.replace(
            "com.puppycrawl.tools.checkstyle.api.CheckstyleException",
            "untrusted.CheckstyleException",
        ),
        "PatternSyntaxException while reading source".to_owned(),
    ] {
        assert_eq!(
            checkstyle_native_failure_reason(Some(254), text.as_bytes()),
            None
        );
    }
    assert_eq!(checkstyle_native_failure_reason(Some(254), &[255]), None);
}
