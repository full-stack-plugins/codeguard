use codeguard_cli::guard_integration::{
    profile::InvocationDescriptor,
    reader::read_native,
    scope::{FrozenObligations, ScopeAssessment, assess_scope},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn valid() -> Value {
    json!({
        "schema_version":"1.0",
        "report_type":"run_report",
        "operation":"lint",
        "request_id":"run-001",
        "run_id":"native-001",
        "command_status":"complete",
        "exit_code":1,
        "request":{
            "schema_version":"1.0",
            "report_type":"check_request",
            "request_id":"run-001",
            "command":"lint",
            "selection":{"language":"python"},
            "root":"/workspace/example",
            "content_source":{"kind":"working_tree"},
            "options":{"format":"json","offline":false,"timeout_ms":30000,"jobs":2}
        },
        "plan_id":"plan-001",
        "identities":{
            "content":{"kind":"working_tree","digest":"a".repeat(64)},
            "policy":{"source":"trusted-local","revision":"r1","digest":"b".repeat(64)},
            "tools":[{"id":"ruff","version":"0.16.8","binary_sha256":"c".repeat(64)}]
        },
        "results":[{
            "obligation_id":"python/app/lint/ruff",
            "completion":"complete",
            "findings":[{
                "id":"f-1",
                "native_rule_id":"F401",
                "tool_id":"ruff",
                "severity":"error",
                "gate_impact":"blocking",
                "message":"unused import",
                "obligation_id":"python/app/lint/ruff",
                "evidence_refs":["run:native-001:ruff"]
            }],
            "coverage":{"expected":["src/main.py"],"observed":["src/main.py"],"unresolved":[],"proof_kind":"native_report"},
            "execution_refs":["run:native-001:ruff"]
        }],
        "delivery_gate":{
            "decision":"not_evaluated",
            "eligible":false,
            "blocking_finding_ids":["f-1"],
            "incomplete_obligation_ids":[],
            "coverage_mismatch_ids":[],
            "invalid_ledger_ids":[]
        },
        "warnings":[],
        "next_actions":["修复 F401 后用原生 Ruff 复检"]
    })
}

fn invocation() -> InvocationDescriptor {
    InvocationDescriptor::parse(br#"{"version":"codeguard.invocation/v1alpha1","package_version":"0.1.4","command":"lint","flags":["--format=json"],"native_schema":"1.0","request_id":"run-001","run_id":"native-001","process_exit":1}"#).unwrap()
}
#[test]
fn rejects_unknown_invocation_fields_versions_and_commands() {
    let base = serde_json::to_value(invocation()).unwrap();
    for (key, value) in [
        ("extra", json!(true)),
        ("version", json!("next")),
        ("command", json!("fix")),
        ("flags", json!(["--repair"])),
    ] {
        let mut v = base.clone();
        v[key] = value;
        assert!(InvocationDescriptor::parse(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}
#[test]
fn retains_raw_bytes_and_findings_without_qualifying_fixture() {
    let bytes = serde_json::to_vec(&valid()).unwrap();
    let evidence = read_native(&bytes, &invocation()).unwrap();
    assert_eq!(evidence.raw_bytes(), bytes);
    assert_eq!(evidence.report().finding_ids, vec!["f-1"]);
    assert_eq!(evidence.raw_sha256().len(), 64);
    let obligations = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    assert!(matches!(
        assess_scope(&evidence, &obligations),
        ScopeAssessment::Partial { .. }
    ));
}
#[test]
fn rejects_stale_run_exit_conflict_unknown_schema_and_truncation() {
    for (key, value) in [
        ("run_id", json!("old-run")),
        ("exit_code", json!(0)),
        ("schema_version", json!("1.4")),
        ("operation", json!("check")),
    ] {
        let mut v = valid();
        v[key] = value;
        assert!(
            read_native(&serde_json::to_vec(&v).unwrap(), &invocation()).is_err(),
            "{key}"
        );
    }
    assert!(read_native(b"{", &invocation()).is_err());
    assert!(
        read_native(
            br#"{"schema_version":"1.0","schema_version":"1.0"}"#,
            &invocation()
        )
        .is_err()
    );
    assert!(read_native(&vec![b' '; 1024 * 1024 + 1], &invocation()).is_err());
}
#[test]
fn frozen_targets_are_not_inferred_from_report_or_unrelated_success() {
    assert!(FrozenObligations::new(BTreeMap::new()).is_err());
    let evidence = read_native(&serde_json::to_vec(&valid()).unwrap(), &invocation()).unwrap();
    for targets in [
        vec!["missing.py".into()],
        vec!["src/main.py".into(), "missing.py".into()],
    ] {
        let obligations =
            FrozenObligations::new(BTreeMap::from([("python/app/lint/ruff".into(), targets)]))
                .unwrap();
        let ScopeAssessment::Partial { gaps } = assess_scope(&evidence, &obligations);
        assert!(gaps.iter().any(|s| s.contains("missing.py")));
    }
}

#[test]
fn direct_deserialization_cannot_bypass_profile_validation() {
    let mut value = serde_json::to_value(invocation()).unwrap();
    value["package_version"] = json!("unregistered");
    let bypass: InvocationDescriptor = serde_json::from_value(value).unwrap();
    assert!(read_native(&serde_json::to_vec(&valid()).unwrap(), &bypass).is_err());
}

#[test]
fn aggregate_incomplete_exit_is_kept_in_scope_diagnostics() {
    let mut report = valid();
    report["command_status"] = json!("incomplete");
    report["exit_code"] = json!(3);
    report["results"][0]["completion"] = json!("incomplete");
    report["results"][0]["reason"] = json!("native_timeout");
    report["delivery_gate"]["incomplete_obligation_ids"] = json!(["python/app/lint/ruff"]);
    let mut descriptor = serde_json::to_value(invocation()).unwrap();
    descriptor["process_exit"] = json!(3);
    let descriptor =
        InvocationDescriptor::parse(&serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let evidence = read_native(&serde_json::to_vec(&report).unwrap(), &descriptor).unwrap();
    let frozen = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let ScopeAssessment::Partial { gaps } = assess_scope(&evidence, &frozen);
    assert!(gaps.iter().any(|gap| gap == "native aggregate incomplete"));
}

#[test]
fn capability_is_explicitly_unqualified_and_distinct_from_native_version() {
    let profile = invocation().capability().unwrap();
    assert_eq!(profile.version, "codeguard.capability/v1alpha1");
    assert_eq!(profile.native_schema, "1.0");
    assert!(!profile.qualified);
    assert_eq!(profile.mapping_version, None);
    assert_eq!(profile.engine_wire_version, None);
}
