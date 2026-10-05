use codeguard_adapters::valid_kotlin_native_observation;
use serde_json::{Value, json};
fn observed() -> Value {
    json!({"status":"diagnostics_observed","reason":"kotlin_native_syntax_diagnostics","version":"kotlinc-jvm 2.4.10","tool_sha256":"a".repeat(64),"tool_identity_scope":"launcher_only","diagnostics":[{"line":1,"column_byte":34,"column_utf16":32,"rule_id":"kotlin.syntax"}],"context_diagnostics":[]})
}
#[test]
fn historical_positions_validate_both_coordinate_units_when_source_is_current() {
    let source = "fun f() { val s = \"😀\"; val x = }\n".as_bytes();
    let report = observed();
    assert!(valid_kotlin_native_observation(&report, Some(source)));
    for column in [31, 33, 34] {
        let mut forged = report.clone();
        forged["diagnostics"][0]["column_utf16"] = json!(column);
        assert!(!valid_kotlin_native_observation(&forged, Some(source)));
    }
    let mut forged = report.clone();
    forged["diagnostics"][0]["column_byte"] = json!(21);
    assert!(!valid_kotlin_native_observation(&forged, Some(source)));
    assert!(valid_kotlin_native_observation(&report, None));
}
#[test]
fn wrong_identity_or_context_cannot_become_native_syntax_evidence() {
    let report = observed();
    for (key, value) in [
        ("version", json!("OTP 28")),
        ("status", json!("completed")),
        ("tool_identity_scope", json!("trusted_toolchain")),
        ("tool_sha256", json!(null)),
    ] {
        let mut forged = report.clone();
        forged[key] = value;
        assert!(!valid_kotlin_native_observation(&forged, None), "{key}");
    }
    let mut forged = report.clone();
    forged["diagnostics"][0]["rule_id"] = json!("kotlin.context.UNRESOLVED_REFERENCE");
    assert!(!valid_kotlin_native_observation(&forged, None));
    let mut duplicate = report.clone();
    duplicate["diagnostics"]
        .as_array_mut()
        .unwrap()
        .push(report["diagnostics"][0].clone());
    assert!(!valid_kotlin_native_observation(&duplicate, None));
    let mut mixed = report.clone();
    mixed["status"] = json!("incomplete");
    mixed["reason"] = json!("kotlin_project_context_unresolved");
    mixed["context_diagnostics"] = json!([{"line":1,"column_byte":31,"column_utf16":29,"rule_id":"kotlin.context.VARIABLE_WITH_NO_TYPE_NO_INITIALIZER"}]);
    assert!(valid_kotlin_native_observation(&mixed, None));
    mixed["context_diagnostics"][0]["rule_id"] = json!("kotlin.context.SYNTAX");
    assert!(!valid_kotlin_native_observation(&mixed, None));
}
