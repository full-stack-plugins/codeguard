fn sample_projection() -> codeguard_cli::guard_integration::projection::Projection {
    let bytes = serde_json::to_vec(&fixture::valid()).unwrap();
    let evidence = read_native(&bytes, &fixture::invocation()).unwrap();
    let contract:GuardContract=serde_json::from_value(json!({"apiVersion":"guard.partme.ai/v1alpha1","kind":"GuardContract","metadata":{"id":"c","revision":"1"},"spec":{"rules":[{"id":"r","enforcement":"advise","assertion":{"type":"forbid_relation","subject":"code","predicate":"has","object":"gap"}}]}})).unwrap();
    let mapping=ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"r"},{"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"r"}]}"#).unwrap();
    project(
        &evidence,
        &obligations(),
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: binding().source_snapshot_digest,
        },
    )
    .unwrap()
}
use codeguard_cli::guard_integration::{
    envelope::FrozenRun,
    profile::InvocationDescriptor,
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::FrozenObligations,
};
use guardengine::{
    Decision, GuardContract, GuardSubject,
    integration::{CoverageStatus, RunBinding, RunStatus, verify_engine_artifacts},
};
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
fn binding() -> RunBinding {
    RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["req".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: format!("sha256:{}", "a".repeat(64)),
        baseline_digest: None,
    }
}
fn freeze(binding: RunBinding) -> Result<FrozenRun, &'static str> {
    FrozenRun::new(
        "native-001",
        binding,
        &obligations(),
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
}
#[test]
fn missing_binding_never_creates_publishable_run() {
    for key in ["repoId", "candidateOid", "baseOid", "sourceSnapshotDigest"] {
        let mut value = serde_json::to_value(binding()).unwrap();
        value[key] = json!("");
        assert!(
            freeze(serde_json::from_value(value).unwrap()).is_err(),
            "{key}"
        );
    }
}
#[test]
fn complete_partial_envelope_is_recomputed_and_bound_to_native_bytes() {
    let bytes = serde_json::to_vec(&fixture::valid()).unwrap();
    let projection = sample_projection();
    let output = freeze(binding()).unwrap().complete(projection).unwrap();
    assert_eq!(output.envelope().decision, Some(Decision::Block));
    assert_eq!(output.envelope().coverage.status, CoverageStatus::Partial);
    assert_eq!(output.domain_bytes(), bytes);
    assert_eq!(
        verify_engine_artifacts(
            output.envelope(),
            output.contract_bytes().unwrap(),
            output.facts_bytes().unwrap(),
            output.report_bytes().unwrap()
        )
        .unwrap()
        .decision,
        Decision::Block
    );
    let mut tampered = output.report_bytes().unwrap().to_vec();
    tampered.push(b' ');
    assert!(
        verify_engine_artifacts(
            output.envelope(),
            output.contract_bytes().unwrap(),
            output.facts_bytes().unwrap(),
            &tampered
        )
        .is_err()
    );
}
#[test]
fn bound_native_failure_and_cancellation_keep_null_decision_and_findings() {
    for (exit, status, expected) in [
        (4, "internal_error", RunStatus::Error),
        (130, "cancelled", RunStatus::Cancelled),
    ] {
        let mut value = fixture::valid();
        value["exit_code"] = json!(exit);
        value["command_status"] = json!(status);
        let mut invocation = serde_json::to_value(fixture::invocation()).unwrap();
        invocation["process_exit"] = json!(exit);
        let invocation =
            InvocationDescriptor::parse(&serde_json::to_vec(&invocation).unwrap()).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        let evidence = read_native(&bytes, &invocation).unwrap();
        let output = freeze(binding()).unwrap().failure(&evidence).unwrap();
        assert_eq!(output.envelope().run_status, expected);
        assert_eq!(output.envelope().decision, None);
        assert!(output.report_bytes().is_none());
        assert_eq!(output.domain_bytes(), bytes);
        assert_eq!(evidence.report().finding_ids, vec!["f-1"]);
    }
}

#[test]
fn changing_frozen_targets_under_same_obligation_id_rejects_publication() {
    let changed = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["other.py".into()],
    )]))
    .unwrap();
    let frozen = FrozenRun::new(
        "native-001",
        binding(),
        &changed,
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap();
    assert!(frozen.complete(sample_projection()).is_err());
}

#[test]
fn native_only_attachment_cannot_satisfy_engine_backed_consumption() {
    use guardengine::integration::EvidenceProfile;
    let output = freeze(binding())
        .unwrap()
        .complete(sample_projection())
        .unwrap();
    let mut weaker = output.envelope().clone();
    weaker.artifacts.contract = None;
    weaker.artifacts.facts = None;
    weaker.artifacts.report = None;
    assert!(weaker.validate(EvidenceProfile::NativeOnly).is_ok());
    assert!(weaker.validate(EvidenceProfile::EngineBacked).is_err());
    assert!(
        verify_engine_artifacts(
            &weaker,
            output.contract_bytes().unwrap(),
            output.facts_bytes().unwrap(),
            output.report_bytes().unwrap(),
        )
        .is_err()
    );
    assert_eq!(weaker.decision, Some(Decision::Block));
    assert_eq!(weaker.coverage.status, CoverageStatus::Partial);
}
