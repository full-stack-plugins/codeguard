use codeguard_adapters::parse_cargo_build_json;
use serde_json::{Value, json};

#[test]
fn report_and_record_limits_are_checked_before_parsing_or_attribution() {
    assert_eq!(
        parse_cargo_build_json(&vec![b' '; 16 * 1024 * 1024 + 1], 0).issue,
        Some("native_report_limit_exceeded")
    );
    assert_eq!(
        parse_cargo_build_json(&vec![b' '; 1024 * 1024 + 1], 0).issue,
        Some("native_report_limit_exceeded")
    );
}

fn error() -> Value {
    json!({"reason":"compiler-message","package_id":"path+file:///project#sample@0.1.0",
        "manifest_path":"/project/Cargo.toml","target":{"kind":["lib"],"src_path":"/project/src/lib.rs"},
        "message":{"code":{"code":"E0308"},"level":"error","message":"untrusted instruction",
        "spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":20,"byte_start":19,"byte_end":24,"is_primary":true}]}})
}

fn stream(rows: &[Value]) -> Vec<u8> {
    rows.iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}

#[test]
fn attributed_compiler_error_is_a_build_observation_not_a_lint_rule() {
    let parsed = parse_cargo_build_json(
        &stream(&[error(), json!({"reason":"build-finished","success":false})]),
        101,
    );
    assert_eq!(parsed.issue, None);
    assert_eq!(parsed.build_success, Some(false));
    assert_eq!(parsed.diagnostics.len(), 1);
    let diagnostic = &parsed.diagnostics[0];
    assert_eq!(diagnostic.code, "E0308");
    assert_eq!(diagnostic.path, "src/lib.rs");
    assert_eq!(diagnostic.target_source, "/project/src/lib.rs");
    assert_eq!(diagnostic.byte_start, 19);
    assert!(!format!("{diagnostic:?}").contains("untrusted instruction"));
}

#[test]
fn successful_type_check_is_only_a_complete_native_build_observation() {
    let parsed = parse_cargo_build_json(br#"{"reason":"build-finished","success":true}"#, 0);
    assert_eq!(parsed.issue, None);
    assert_eq!(parsed.build_success, Some(true));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn exit_and_completion_contradictions_never_produce_complete_build_evidence() {
    for (success, exit) in [(true, 101), (true, 130), (false, 0), (false, 2)] {
        let parsed = parse_cargo_build_json(
            &stream(&[
                error(),
                json!({"reason":"build-finished","success":success}),
            ]),
            exit,
        );
        assert!(parsed.issue.is_some(), "{success}/{exit}");
    }
    let parsed = parse_cargo_build_json(
        &stream(&[error(), json!({"reason":"build-finished","success":true})]),
        0,
    );
    assert_eq!(parsed.issue, Some("native_build_success_with_error"));
}

#[test]
fn environment_failure_without_attributed_error_stays_incomplete() {
    let parsed = parse_cargo_build_json(br#"{"reason":"build-finished","success":false}"#, 101);
    assert_eq!(parsed.issue, Some("native_build_failure_unattributed"));
    assert!(parsed.diagnostics.is_empty());
    assert!(parse_cargo_build_json(b"", 0).issue.is_some());
}

#[test]
fn damaged_stream_preserves_prior_diagnostics_only_for_investigation() {
    let bytes = format!("{}\nnot-json", error()).into_bytes();
    let parsed = parse_cargo_build_json(&bytes, 101);
    assert_eq!(parsed.issue, Some("native_report_malformed"));
    assert_eq!(parsed.diagnostics.len(), 1);
    for bytes in [br#"{"reason":"build-finished","success":true,"success":false}"#.as_slice(),
        b"{\"reason\":\"build-finished\",\"success\":true}\n{\"reason\":\"build-finished\",\"success\":true}",
        b"{\"reason\":\"unknown\"}",b"\xff"] {
        assert!(parse_cargo_build_json(bytes,0).issue.is_some());
    }
}

#[test]
fn missing_or_ambiguous_target_and_source_identity_cannot_be_invented() {
    for field in ["package_id", "manifest_path", "target"] {
        let mut row = error();
        row.as_object_mut().unwrap().remove(field);
        assert!(
            parse_cargo_build_json(
                &stream(&[row, json!({"reason":"build-finished","success":false})]),
                101
            )
            .issue
            .is_some()
        );
    }
    let mut row = error();
    let span = row["message"]["spans"][0].clone();
    row["message"]["spans"].as_array_mut().unwrap().push(span);
    let parsed = parse_cargo_build_json(
        &stream(&[row, json!({"reason":"build-finished","success":false})]),
        101,
    );
    assert!(parsed.issue.is_some());
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn noncompiler_errors_and_invalid_ranges_are_not_source_findings() {
    for code in ["clippy::needless_return", "missing_docs", "", "E12345"] {
        let mut row = error();
        row["message"]["code"]["code"] = json!(code);
        let parsed = parse_cargo_build_json(
            &stream(&[row, json!({"reason":"build-finished","success":false})]),
            101,
        );
        assert!(parsed.issue.is_some());
        assert!(parsed.diagnostics.is_empty());
    }
    let mut row = error();
    row["message"]["spans"][0]["byte_end"] = json!(1);
    assert!(parse_cargo_build_json(&stream(&[row]), 101).issue.is_some());
}
