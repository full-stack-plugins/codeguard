use codeguard_adapters::parse_cargo_rustdoc_json;
use serde_json::{Value, json};

fn diagnostic(rule: &str) -> Value {
    json!({"reason":"compiler-message","package_id":"path+file:///project#sample@0.1.0",
        "manifest_path":"/project/Cargo.toml","target":{"kind":["lib"],"src_path":"/project/src/lib.rs"},
        "message":{"code":{"code":rule},"level":"warning","message":"untrusted raw text",
            "spans":[{"file_name":"src/lib.rs","line_start":2,"column_start":1,"byte_start":0,"byte_end":22,"is_primary":true}]}})
}

fn stream(rows: &[Value]) -> Vec<u8> {
    rows.iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}

#[test]
fn documentation_rules_preserve_native_target_and_location_without_raw_text() {
    let parsed = parse_cargo_rustdoc_json(&stream(&[
        diagnostic("missing_docs"),
        diagnostic("rustdoc::broken_intra_doc_links"),
        json!({"reason":"build-finished","success":true}),
    ]));
    assert_eq!(parsed.issue, None);
    assert!(parsed.build_finished);
    assert_eq!(parsed.findings.len(), 2);
    assert_eq!(parsed.findings[0].rule_id, "missing_docs");
    assert_eq!(parsed.findings[0].path, "src/lib.rs");
    assert_eq!(parsed.findings[0].line, 2);
    assert_eq!(parsed.findings[0].manifest_path, "/project/Cargo.toml");
    assert_eq!(parsed.findings[0].target_source, "/project/src/lib.rs");
    assert!(!format!("{:?}", parsed.findings).contains("untrusted raw text"));
}

#[test]
fn unsupported_warning_and_compilation_error_are_not_documentation_findings() {
    let mut error = diagnostic("E0433");
    error["message"]["level"] = json!("error");
    let parsed = parse_cargo_rustdoc_json(&stream(&[
        error,
        json!({"reason":"build-finished","success":false}),
    ]));
    assert_eq!(parsed.issue, Some("non_rustdoc_compilation_error"));
    assert!(parsed.findings.is_empty());
    let parsed = parse_cargo_rustdoc_json(&stream(&[
        diagnostic("clippy::needless_return"),
        json!({"reason":"build-finished","success":true}),
    ]));
    assert_eq!(parsed.issue, Some("unsupported_rustdoc_warning"));
    assert!(parsed.findings.is_empty());
}

#[test]
fn incomplete_stream_preserves_observed_findings_without_claiming_completion() {
    for ending in [
        None,
        Some(json!({"reason":"build-finished","success":false})),
    ] {
        let mut rows = vec![diagnostic("missing_docs")];
        rows.extend(ending);
        let parsed = parse_cargo_rustdoc_json(&stream(&rows));
        assert!(parsed.issue.is_some());
        assert!(!parsed.build_finished);
        assert_eq!(parsed.findings.len(), 1);
    }
}

#[test]
fn duplicate_keys_and_after_finish_records_are_rejected() {
    let parsed =
        parse_cargo_rustdoc_json(br#"{"reason":"build-finished","success":false,"success":true}"#);
    assert_eq!(parsed.issue, Some("native_report_malformed"));
    for row in [
        diagnostic("missing_docs"),
        json!({"reason":"build-finished","success":true}),
    ] {
        let parsed = parse_cargo_rustdoc_json(&stream(&[
            json!({"reason":"build-finished","success":true}),
            row,
        ]));
        assert_eq!(parsed.issue, Some("native_report_event_after_finish"));
    }
}

#[test]
fn ambiguous_or_zero_primary_locations_are_rejected() {
    for count in [0, 2] {
        let mut row = diagnostic("missing_docs");
        let span = row["message"]["spans"][0].clone();
        row["message"]["spans"] = json!(vec![span; count]);
        let parsed = parse_cargo_rustdoc_json(&stream(&[
            row,
            json!({"reason":"build-finished","success":true}),
        ]));
        assert_eq!(parsed.issue, Some("native_finding_location_ambiguous"));
        assert!(parsed.findings.is_empty());
    }
    let mut row = diagnostic("missing_docs");
    row["message"]["spans"][0]["line_start"] = json!(0);
    assert!(parse_cargo_rustdoc_json(&stream(&[row])).issue.is_some());
}

#[test]
fn missing_native_target_identity_cannot_be_invented_from_diagnostic_text() {
    for field in ["package_id", "manifest_path", "target"] {
        let mut row = diagnostic("missing_docs");
        row.as_object_mut().unwrap().remove(field);
        let parsed = parse_cargo_rustdoc_json(&stream(&[row]));
        assert_eq!(parsed.issue, Some("native_target_identity_missing"));
        assert!(parsed.findings.is_empty());
    }
}

#[test]
fn malformed_utf8_unknown_records_and_success_without_diagnostics_are_distinct() {
    assert_eq!(
        parse_cargo_rustdoc_json(&[0xff]).issue,
        Some("native_report_non_utf8")
    );
    assert_eq!(
        parse_cargo_rustdoc_json(b"not-json").issue,
        Some("native_report_malformed")
    );
    assert_eq!(
        parse_cargo_rustdoc_json(br#"{"reason":"unknown"}"#).issue,
        Some("native_report_unknown_record")
    );
    let parsed = parse_cargo_rustdoc_json(br#"{"reason":"build-finished","success":true}"#);
    assert_eq!(parsed.issue, None);
    assert!(parsed.build_finished);
    assert!(parsed.findings.is_empty());
}
