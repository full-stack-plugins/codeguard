use codeguard_cli::guard_integration::{
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::{FrozenObligations, ScopeAssessment, assess_scope},
};
use guardengine::{Completeness, Decision, GuardContract, GuardSubject};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[path = "support/guard_integration.rs"]
mod fixture;

fn assert_unqualified_block(report: Value) {
    let mut invocation = serde_json::to_value(fixture::invocation()).unwrap();
    invocation["process_exit"] = report["exit_code"].clone();
    let invocation = codeguard_cli::guard_integration::profile::InvocationDescriptor::parse(
        &serde_json::to_vec(&invocation).unwrap(),
    )
    .unwrap();
    assert!(!invocation.capability().unwrap().qualified);
    let bytes = serde_json::to_vec(&report).unwrap();
    let evidence = read_native(&bytes, &invocation).unwrap();
    let obligations = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let ScopeAssessment::Partial { gaps } = assess_scope(&evidence, &obligations);
    assert_eq!(gaps, vec!["native profile unqualified"]);
    let contract: GuardContract = serde_json::from_value(json!({
        "apiVersion": guardengine::API_VERSION, "kind":"GuardContract",
        "metadata":{"id":"qualification-control","revision":"1"},
        "spec":{"rules":[{"id":"gap","enforcement":"advise","assertion":{
            "type":"forbid_relation","subject":"native","predicate":"lacks","object":"qualification"
        }}]}
    }))
    .unwrap();
    let mapping = ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"gap"}]}"#).unwrap();
    let projected = project(
        &evidence,
        &obligations,
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: format!("sha256:{}", "a".repeat(64)),
        },
    )
    .unwrap();
    assert_eq!(projected.facts().completeness, Completeness::Partial);
    assert_eq!(projected.report().decision, Decision::Block);
    assert_eq!(projected.domain_bytes(), bytes);
}
fn empty_success() -> Value {
    let mut report = fixture::valid();
    report["exit_code"] = json!(0);
    report["results"][0]["findings"] = json!([]);
    report["delivery_gate"]["blocking_finding_ids"] = json!([]);
    report
}
#[test]
fn zero_findings_and_native_exit_zero_cannot_qualify_complete() {
    assert_unqualified_block(empty_success());
}
#[test]
fn wasm_only_candidate_inventory_cannot_qualify_complete() {
    let mut report = empty_success();
    // Even a structurally accepted inventory claiming complete targets cannot
    // qualify a non-native candidate as native execution evidence.
    report["identities"]["tools"][0]["id"] = json!("wasm-precheck-candidate");
    report["results"][0]["coverage"]["proof_kind"] = json!("file_manifest");
    report["results"][0]["execution_refs"] = json!(["wasm:syntax-candidate"]);
    assert_unqualified_block(report.clone());
    report["results"][0]["coverage"]["proof_kind"] = json!("wasm");
    assert!(
        read_native(
            &serde_json::to_vec(&report).unwrap(),
            &fixture::invocation()
        )
        .is_err()
    );
}
#[test]
fn resolved_task_messages_cannot_qualify_complete_or_replace_native_report() {
    let mut report = empty_success();
    report["warnings"] = json!(["task task-1 resolved"]);
    report["next_actions"] = json!(["task resolution recorded; no remaining findings"]);
    assert_unqualified_block(report);
    let task_result = json!({"schema_version":"0.6.0","report_type":"task_verification_preview",
        "operation":"task_verify","observation":"resolved","delivery_decision":"not_evaluated"});
    assert!(
        read_native(
            &serde_json::to_vec(&task_result).unwrap(),
            &fixture::invocation()
        )
        .is_err()
    );
}
#[test]
fn qualification_fixture_lists_only_unqualified_profiles_and_legacy_dependencies() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/guard-integration/qualification.json"
    ))
    .unwrap();
    assert_eq!(fixture["version"], "codeguard.qualification/v1alpha1");
    let profiles = fixture["profiles"].as_array().unwrap();
    assert!(!profiles.is_empty());
    for profile in profiles {
        assert_eq!(profile["qualified"], false);
        assert_eq!(profile["qualification_status"], "unverified");
        for dependency in profile["native_task_dependencies"].as_array().unwrap() {
            let id = dependency.as_str().unwrap();
            assert!(
                fixture["native_task_requirements"][id]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            );
        }
    }
}
