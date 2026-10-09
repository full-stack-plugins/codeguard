use codeguard_cli::guard_integration::{
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::FrozenObligations,
};
use guardengine::{Decision, GuardContract, GuardSubject, RuleStatus};
use serde_json::json;
use std::collections::BTreeMap;
#[path = "support/guard_integration.rs"]
mod fixture;
fn obligations() -> FrozenObligations {
    FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap()
}
fn contract(mode: &str) -> GuardContract {
    serde_json::from_value(json!({"apiVersion":"guard.partme.ai/v1alpha1","kind":"GuardContract","metadata":{"id":"protected","revision":"1"},"spec":{"rules":[{"id":"finding","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"code","predicate":"has","object":"F401"}},{"id":"gap","enforcement":mode,"assertion":{"type":"forbid_relation","subject":"native","predicate":"lacks","object":"qualification"}}]}})).unwrap()
}
fn mapping() -> serde_json::Value {
    json!({"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"finding"},{"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"gap"}]})
}
fn subject() -> GuardSubject {
    GuardSubject {
        id: "repo".into(),
        snapshot_digest: format!("sha256:{}", "a".repeat(64)),
    }
}
#[test]
fn partial_projection_uses_engine_for_every_enforcement_and_keeps_attachment() {
    let bytes = serde_json::to_vec(&fixture::valid()).unwrap();
    let evidence = read_native(&bytes, &fixture::invocation()).unwrap();
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&mapping()).unwrap()).unwrap();
    for mode in ["enforce", "review", "advise"] {
        let projection = project(
            &evidence,
            &obligations(),
            &mapping,
            &contract(mode),
            subject(),
        )
        .unwrap();
        assert_eq!(projection.report().decision, Decision::Block);
        assert!(
            projection
                .report()
                .evaluations
                .iter()
                .all(|e| e.status == RuleStatus::Indeterminate)
        );
        assert_eq!(projection.domain_bytes(), bytes);
        assert_eq!(projection.facts().facts.len(), 2);
    }
}
#[test]
fn missing_mapping_unknown_fields_and_wildcards_cannot_drop_findings() {
    let evidence = read_native(
        &serde_json::to_vec(&fixture::valid()).unwrap(),
        &fixture::invocation(),
    )
    .unwrap();
    for index in 0..2 {
        let mut value = mapping();
        value["entries"].as_array_mut().unwrap().remove(index);
        let mapping = ProtectedMapping::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            project(
                &evidence,
                &obligations(),
                &mapping,
                &contract("advise"),
                subject()
            )
            .is_err()
        );
    }
    let mut value = mapping();
    value["count"] = json!(0);
    assert!(ProtectedMapping::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut contract = contract("enforce");
    contract.spec.rules[0].assertion = guardengine::GuardAssertion::ForbidRelation {
        subject: "*".into(),
        predicate: "has".into(),
        object: "F401".into(),
    };
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&mapping()).unwrap()).unwrap();
    assert!(project(&evidence, &obligations(), &mapping, &contract, subject()).is_err());
}
#[test]
fn source_snapshot_conflict_rejects_projection() {
    let evidence = read_native(
        &serde_json::to_vec(&fixture::valid()).unwrap(),
        &fixture::invocation(),
    )
    .unwrap();
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&mapping()).unwrap()).unwrap();
    let mut subject = subject();
    subject.snapshot_digest = format!("sha256:{}", "b".repeat(64));
    assert!(
        project(
            &evidence,
            &obligations(),
            &mapping,
            &contract("enforce"),
            subject
        )
        .is_err()
    );
}

#[test]
fn unused_mapping_to_unknown_rule_is_rejected() {
    let evidence = read_native(
        &serde_json::to_vec(&fixture::valid()).unwrap(),
        &fixture::invocation(),
    )
    .unwrap();
    let mut value = mapping();
    value["entries"].as_array_mut().unwrap().push(json!({"source":{"kind":"finding","tool_id":"other","native_rule_id":"other"},"rule_id":"missing"}));
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        project(
            &evidence,
            &obligations(),
            &mapping,
            &contract("advise"),
            subject()
        )
        .is_err()
    );
}
