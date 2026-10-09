use codeguard_cli::guard_integration::{
    consumer::{ExpectedConsumption, consume},
    envelope::{EnvelopeOutput, FrozenRun},
    projection::ProtectedMapping,
    ruff_profile::{RuffF401Policy, project_ruff, read_ruff_feedback},
};
use guardengine::{
    GuardContract, GuardSubject,
    integration::{GuardRunEnvelope, RunBinding, eligibility::*},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
pub(crate) fn sample(review: bool) -> (EnvelopeOutput, ExpectedConsumption) {
    sample_native(if review { "bad" } else { "clean" }, review)
}
pub(crate) fn sample_native(name: &str, review: bool) -> (EnvelopeOutput, ExpectedConsumption) {
    sample_with_requirements(name, review, vec!["R".into()])
}
pub(crate) fn sample_with_requirements(
    name: &str,
    review: bool,
    requirements: Vec<String>,
) -> (EnvelopeOutput, ExpectedConsumption) {
    let raw = fixture(&format!("ruff-f401/{name}.json"));
    let native: Value = serde_json::from_slice(&raw).unwrap();
    let capture: Value = serde_json::from_slice(&fixture("ruff-f401/capture.json")).unwrap();
    let producer = capture["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        capture["cases"][name]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let run = native["run_id"].as_str().unwrap();
    let evidence = read_ruff_feedback(&raw, &policy, run, 3, producer).unwrap();
    let mut vocab: Value = serde_json::from_slice(&fixture("relations.json")).unwrap();
    if review {
        vocab["contract"]["spec"]["rules"][0]["enforcement"] = json!("review");
    }
    let mapping = ProtectedMapping::parse(&serde_json::to_vec(&vocab["mapping"]).unwrap()).unwrap();
    let contract: GuardContract = serde_json::from_value(vocab["contract"].clone()).unwrap();
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: requirements,
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: policy.source_digest(),
        baseline_digest: None,
    };
    let projection = project_ruff(
        &evidence,
        &mapping,
        &contract,
        GuardSubject {
            id: "repo".into(),
            snapshot_digest: policy.source_digest(),
        },
    )
    .unwrap();
    let output = FrozenRun::new(
        run,
        binding,
        &policy.obligations(),
        "2026-10-09T00:00:00Z",
        "2026-10-09T00:01:00Z",
    )
    .unwrap()
    .complete(projection)
    .unwrap();
    let e = output.envelope();
    let expected = ExpectedConsumption {
        policy: EligibilityPolicy {
            binding: e.binding.clone(),
            producer: e.producer.clone(),
            required_scopes: e.coverage.required_scopes.clone(),
            contract_digest: e.artifacts.contract.as_ref().unwrap().digest.clone(),
            action: "merge".into(),
            producer_principals: BTreeSet::from(["fixture-producer".into()]),
            approval_principals: BTreeMap::from([(
                "review".into(),
                BTreeSet::from(["fixture-reviewer".into()]),
            )]),
        },
        run_id: run.into(),
        mapping_digest: mapping.digest(),
        envelope_digest: sha(&serde_json::to_vec(e).unwrap()),
        raw_domain_digest: sha(&raw),
    };
    (output, expected)
}
// Explicit test provider only; this is not a production identity implementation.
pub(crate) struct FixtureProvider {
    calls: Cell<usize>,
    revoked: Cell<bool>,
    unavailable: Cell<bool>,
}
impl FixtureProvider {
    pub(crate) fn new() -> Self {
        Self {
            calls: Cell::new(0),
            revoked: Cell::new(false),
            unavailable: Cell::new(false),
        }
    }
}
impl AuthorityProvider for FixtureProvider {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        Ok(ProducerRecord {
            principal: "fixture-producer".into(),
            producer: e.producer.clone(),
            envelope_digest: d.into(),
            validity: Validity {
                issued_at: 0,
                expires_at: 2_000_000_000,
                revoked: self.revoked.get(),
            },
        })
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
pub(crate) const NOW: i64 = 1_791_504_120;
#[test]
fn each_frozen_binding_mapping_run_domain_contract_analyzer_and_scope_change_invalidates() {
    let (output, expected) = sample(false);
    let provider = FixtureProvider::new();
    assert!(
        consume(&output, &expected, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    for field in [
        "repoId",
        "taskId",
        "worktreeId",
        "requirementIds",
        "candidateOid",
        "baseOid",
        "mergeGroupId",
        "sourceSnapshotDigest",
        "baselineDigest",
    ] {
        let mut changed = expected.clone();
        let mut binding = serde_json::to_value(&changed.policy.binding).unwrap();
        binding[field] = if field == "requirementIds" {
            json!(["other"])
        } else {
            json!("changed")
        };
        changed.policy.binding = serde_json::from_value(binding).unwrap();
        assert!(
            !consume(&output, &changed, &provider, NOW, None)
                .unwrap()
                .eligibility
                .eligible,
            "{field}"
        );
    }
    for n in 0..10 {
        let mut changed = expected.clone();
        match n {
            0 => changed.run_id.push('x'),
            1 => changed.mapping_digest.push('x'),
            2 => changed.envelope_digest.push('x'),
            3 => changed.raw_domain_digest.push('x'),
            4 => changed.policy.contract_digest.push('x'),
            5 => changed.policy.producer.analyzer_id.push('x'),
            6 => changed.policy.producer.analyzer_version.push('x'),
            7 => changed.policy.required_scopes[0].push('x'),
            8 => changed.policy.producer.version.push('x'),
            _ => {
                changed.policy.producer_principals = BTreeSet::from(["other".into()]);
            }
        }
        assert!(
            consume(&output, &changed, &provider, NOW, None)
                .map_or(true, |c| !c.eligibility.eligible),
            "case {n}"
        );
    }
}
#[test]
fn every_call_requeries_authority_and_preserves_original_report() {
    let (output, expected) = sample(false);
    let provider = FixtureProvider::new();
    let report = output.report_bytes().unwrap().to_vec();
    let first = consume(&output, &expected, &provider, NOW, None).unwrap();
    for _ in 0..10 {
        assert_eq!(
            consume(&output, &expected, &provider, NOW, None)
                .unwrap()
                .freshness_key,
            first.freshness_key
        );
    }
    assert_eq!(provider.calls.get(), 11);
    provider.revoked.set(true);
    let revoked = consume(&output, &expected, &provider, NOW, None).unwrap();
    assert!(!revoked.eligibility.eligible);
    assert_ne!(revoked.freshness_key, first.freshness_key);
    assert_eq!(
        revoked.eligibility.technical_decision,
        first.eligibility.technical_decision
    );
    provider.unavailable.set(true);
    assert_eq!(
        consume(&output, &expected, &provider, NOW, None)
            .unwrap()
            .eligibility
            .code,
        EligibilityCode::ProviderUnavailable
    );
    assert_eq!(output.report_bytes().unwrap(), report);
    provider.revoked.set(false);
    provider.unavailable.set(false);
    assert_eq!(
        consume(&output, &expected, &provider, 2_000_000_001, None)
            .unwrap()
            .eligibility
            .code,
        EligibilityCode::UntrustedProducer
    );
}
#[test]
fn missing_approval_never_becomes_allow_and_budget_precedes_provider() {
    let (output, mut expected) = sample(true);
    let provider = FixtureProvider::new();
    let result = consume(&output, &expected, &provider, NOW, None).unwrap();
    assert_eq!(result.eligibility.code, EligibilityCode::MissingApproval);
    assert_eq!(
        result.eligibility.technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    expected.policy.action = "x".repeat(1_048_577);
    let before = provider.calls.get();
    assert!(consume(&output, &expected, &provider, NOW, None).is_err());
    assert_eq!(before, provider.calls.get());
}

#[test]
fn partial_scope_preserves_block_without_querying_provider() {
    let (output, expected) = sample_native("missing-tool", false);
    let provider = FixtureProvider::new();
    let report = output.report_bytes().unwrap().to_vec();
    let consumed = consume(&output, &expected, &provider, NOW, None).unwrap();
    assert_eq!(consumed.eligibility.code, EligibilityCode::Incomplete);
    assert_eq!(
        consumed.eligibility.technical_decision,
        Some(guardengine::Decision::Block)
    );
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(report, output.report_bytes().unwrap());
}
