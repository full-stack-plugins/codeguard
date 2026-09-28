use codeguard_core::{Finding, FindingLocation, GateImpact};

#[test]
fn missing_policy_impact_cannot_be_deserialized_as_a_pass() {
    let input = r#"{"id":"f-1","native_rule_id":"F401","tool_id":"ruff","severity":"error","message":"unused import","obligation_id":"python/lint"}"#;
    assert!(serde_json::from_str::<Finding>(input).is_err());
}

#[test]
fn unknown_impact_value_is_rejected() {
    let input = r#"{"id":"f-1","native_rule_id":"F401","tool_id":"ruff","severity":"UNKNOWN","gate_impact":"ignore","message":"unused import","obligation_id":"python/lint"}"#;
    assert!(serde_json::from_str::<Finding>(input).is_err());
}

#[test]
fn native_severity_and_policy_impact_remain_distinct() {
    let input = r#"{"id":"cve-1","native_rule_id":"CVE-2026-1","tool_id":"osv-scanner","severity":"UNKNOWN","gate_impact":"blocking","message":"advisory match","obligation_id":"rust/cve"}"#;
    let finding: Finding = serde_json::from_str(input).expect("explicit policy impact");
    assert_eq!(finding.severity, "UNKNOWN");
    assert_eq!(finding.gate_impact, GateImpact::Blocking);
}

#[test]
fn source_location_serializes_with_kind_and_one_based_coordinates() {
    let location = FindingLocation::Source {
        path: "src/main.py".into(),
        line: Some(3),
        column: Some(7),
    };
    let encoded = serde_json::to_value(&location).expect("位置可序列化");
    assert_eq!(encoded["kind"], "source");
    assert_eq!(encoded["path"], "src/main.py");
    assert_eq!(encoded["line"], 3);
    assert_eq!(encoded["column"], 7);
    let decoded: FindingLocation = serde_json::from_value(encoded).expect("位置可反序列化");
    assert_eq!(decoded, location);
}
