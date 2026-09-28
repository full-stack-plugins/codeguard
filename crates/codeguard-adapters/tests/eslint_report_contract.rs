use codeguard_adapters::parse_eslint_json;
use serde_json::{Value, json};
fn message(rule: Option<&str>, severity: u64, fatal: bool) -> Value {
    json!({"ruleId":rule,"severity":severity,"fatal":fatal,"message":"native diagnostic","line":1,"column":2})
}
fn file(messages: Vec<Value>, errors: u64, warnings: u64, fatal: u64) -> Value {
    json!({"filePath":"/workspace/app.js","messages":messages,"suppressedMessages":[],"errorCount":errors,"warningCount":warnings,"fatalErrorCount":fatal,"fixableErrorCount":0,"fixableWarningCount":0})
}
fn parse(
    value: Value,
    exit: Option<i32>,
    threshold: Option<u64>,
) -> codeguard_adapters::EslintParsed {
    parse_eslint_json(
        &serde_json::to_vec(&json!([value])).unwrap(),
        "10.0.0",
        "10.0.0",
        &["/workspace/app.js".into()],
        exit,
        threshold,
    )
}
#[test]
fn warning_exit_zero_and_max_warning_exit_one_keep_native_findings() {
    let value = file(vec![message(Some("semi"), 1, false)], 0, 1, 0);
    for (exit, threshold) in [(Some(0), None), (Some(1), Some(0)), (Some(0), Some(1))] {
        let result = parse(value.clone(), exit, threshold);
        assert!(result.local_coherent, "{:?}", result.reason);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].severity, 1);
    }
    assert!(!parse(value, Some(1), None).local_coherent);
}
#[test]
fn normal_errors_require_native_exit_one_and_never_treat_exit_two_as_lint() {
    let value = file(vec![message(Some("no-undef"), 2, false)], 1, 0, 0);
    assert!(parse(value.clone(), Some(1), None).local_coherent);
    for exit in [None, Some(0), Some(2)] {
        let result = parse(value.clone(), exit, None);
        assert!(!result.local_coherent);
        assert!(result.findings.is_empty());
    }
}
#[test]
fn fatal_or_unattributed_messages_require_investigation_and_preserve_valid_rules() {
    let result = parse(
        file(
            vec![message(Some("semi"), 2, false), message(None, 2, true)],
            2,
            0,
            1,
        ),
        Some(1),
        None,
    );
    assert!(!result.local_coherent);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(
        result.reason,
        Some("eslint_parser_or_configuration_diagnostic")
    );
    let result = parse(file(vec![message(None, 1, false)], 0, 1, 0), Some(0), None);
    assert!(!result.local_coherent);
    assert!(result.findings.is_empty());
    assert_eq!(result.reason, Some("eslint_unattributed_diagnostic"));
}
#[test]
fn suppression_is_visible_but_does_not_become_a_whitelist_or_clean_scope() {
    let mut value = file(vec![], 0, 0, 0);
    value["suppressedMessages"] = json!([message(Some("no-undef"), 2, false)]);
    let result = parse(value, Some(0), None);
    assert!(!result.local_coherent);
    assert_eq!(result.suppressed_count, 1);
    assert!(result.findings.is_empty());
    assert_eq!(result.reason, Some("eslint_suppression_requires_review"));
}
#[test]
fn counters_scope_duplicates_and_bad_json_cannot_produce_clean_observations() {
    let clean = file(vec![], 0, 0, 0);
    assert!(parse(clean.clone(), Some(0), None).local_coherent);
    for value in [
        {
            let mut v = clean.clone();
            v["errorCount"] = json!(1);
            v
        },
        {
            let mut v = clean.clone();
            v["filePath"] = json!("/outside/app.js");
            v
        },
        {
            let mut v = clean.clone();
            v["filePath"] = json!("/workspace/../app.js");
            v
        },
    ] {
        assert!(!parse(value, Some(0), None).local_coherent);
    }
    for bytes in [
        b"[]".as_slice(),
        b"not json",
        br#"[{"filePath":"/workspace/app.js","filePath":"/outside/app.js"}]"#,
    ] {
        assert!(
            !parse_eslint_json(
                bytes,
                "10.0.0",
                "10.0.0",
                &["/workspace/app.js".into()],
                Some(0),
                None
            )
            .local_coherent
        );
    }
    assert!(
        !parse_eslint_json(
            &serde_json::to_vec(&json!([clean.clone(), clean])).unwrap(),
            "10.0.0",
            "10.0.0",
            &["/workspace/app.js".into()],
            Some(0),
            None
        )
        .local_coherent
    );
}
#[test]
fn version_and_empty_expected_scope_are_not_proven_by_a_json_array() {
    let bytes = serde_json::to_vec(&json!([file(vec![], 0, 0, 0)])).unwrap();
    for (expected, observed, files) in [
        ("10.0.0", "10.1.0", vec!["/workspace/app.js".into()]),
        ("9.0.0", "9.0.0", vec!["/workspace/app.js".into()]),
        ("10.0.0", "10.0.0", vec![]),
    ] {
        assert!(
            !parse_eslint_json(&bytes, expected, observed, &files, Some(0), None).local_coherent
        );
    }
}

#[test]
fn duplicate_semantic_fields_and_relative_segments_are_rejected() {
    let clean = file(vec![], 0, 0, 0);
    let bytes = serde_json::to_string(&json!([clean]))
        .unwrap()
        .replace("\"errorCount\":0", "\"errorCount\":0,\"errorCount\":0");
    assert!(
        !parse_eslint_json(
            bytes.as_bytes(),
            "10.0.0",
            "10.0.0",
            &["/workspace/app.js".into()],
            Some(0),
            None
        )
        .local_coherent
    );
    let mut v = file(vec![], 0, 0, 0);
    v["filePath"] = json!("/workspace/./app.js");
    assert!(
        !parse_eslint_json(
            &serde_json::to_vec(&json!([v])).unwrap(),
            "10.0.0",
            "10.0.0",
            &["/workspace/./app.js".into()],
            Some(0),
            None
        )
        .local_coherent
    );
}
