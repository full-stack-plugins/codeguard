use codeguard_adapters::{RuffParseState, parse_ruff_json};

const F401: &str = r#"[{"code":"F401","message":"`os` imported but unused","filename":"/fixture/unused.py","location":{"row":1,"column":8},"end_location":{"row":1,"column":10},"severity":"error","fix":null,"cell":null}]"#;
const E902: &str = r#"[{"code":"E902","message":"No such file or directory (os error 2)","filename":"/fixture/missing.py","location":{"row":1,"column":1},"end_location":{"row":1,"column":1},"severity":"error"}]"#;

#[test]
fn valid_native_exit_one_preserves_rule_and_location() {
    let parsed = parse_ruff_json(1, F401.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Valid);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].code, "F401");
    assert_eq!(parsed.diagnostics[0].filename, "/fixture/unused.py");
    assert_eq!(parsed.diagnostics[0].location.row, 1);
}

#[test]
fn valid_empty_report_requires_native_exit_zero() {
    let parsed = parse_ruff_json(0, b"[]");
    assert_eq!(parsed.state, RuffParseState::Valid);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn configuration_exit_is_incomplete_not_a_code_violation() {
    let parsed = parse_ruff_json(2, b"");
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn crash_after_complete_json_keeps_valid_diagnostics_but_is_incomplete() {
    let parsed = parse_ruff_json(2, F401.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn contradictory_exit_cannot_pass_or_claim_complete_violation() {
    let parsed = parse_ruff_json(0, F401.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert_eq!(parsed.diagnostics.len(), 1);
    let parsed = parse_ruff_json(1, b"[]");
    assert_eq!(parsed.state, RuffParseState::Incomplete);
}

#[test]
fn malformed_or_schema_invalid_json_is_incomplete() {
    for output in [
        b"[{".as_slice(),
        b"[\xff]".as_slice(),
        br#"[{"code":"F401","message":"x"}]"#.as_slice(),
        br#"{"code":"F401"}"#.as_slice(),
    ] {
        let parsed = parse_ruff_json(1, output);
        assert_eq!(parsed.state, RuffParseState::Incomplete);
    }
}

#[test]
fn valid_entries_survive_an_invalid_sibling_entry() {
    let mixed = format!("[{},{{\"code\":\"E999\"}}]", &F401[1..F401.len() - 1]);
    let parsed = parse_ruff_json(1, mixed.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].code, "F401");
}

#[test]
fn unknown_native_severity_is_preserved() {
    let unknown = F401.replace("\"severity\":\"error\"", "\"severity\":\"UNKNOWN\"");
    let parsed = parse_ruff_json(1, unknown.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Valid);
    assert_eq!(parsed.diagnostics[0].severity, "UNKNOWN");
}

#[test]
fn native_io_diagnostic_is_an_environment_failure_not_a_source_finding() {
    let parsed = parse_ruff_json(1, E902.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(parsed.environment_diagnostics.len(), 1);
    assert_eq!(parsed.environment_diagnostics[0].code, "E902");
    assert_eq!(parsed.reason, Some("native_io_error"));
}

#[test]
fn valid_finding_survives_a_sibling_io_failure() {
    let mixed = format!(
        "[{},{}]",
        &F401[1..F401.len() - 1],
        &E902[1..E902.len() - 1]
    );
    let parsed = parse_ruff_json(1, mixed.as_bytes());
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].code, "F401");
    assert_eq!(parsed.environment_diagnostics.len(), 1);
}
