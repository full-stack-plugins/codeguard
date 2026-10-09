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
    cell::{Cell, RefCell},
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
fn sample(name: &str, review: bool, baseline: bool) -> (EnvelopeOutput, ExpectedConsumption) {
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
        requirement_ids: vec!["R".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: policy.source_digest(),
        baseline_digest: baseline.then(|| format!("sha256:{}", "e".repeat(64))),
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

const NOW: i64 = 1_791_504_120;
// Explicit fixture authority; producer record alone is not baseline approval.
struct Authority {
    expected: ExpectedConsumption,
    approvals: RefCell<BTreeMap<String, ApprovalRecord>>,
    unavailable: Cell<bool>,
    calls: Cell<usize>,
}
impl Authority {
    fn new(expected: &ExpectedConsumption) -> Self {
        Self {
            expected: expected.clone(),
            approvals: RefCell::new(BTreeMap::new()),
            unavailable: Cell::new(false),
            calls: Cell::new(0),
        }
    }
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if d != self.expected.envelope_digest {
            return Err(AuthorityError::Untrusted);
        }
        Ok(ProducerRecord {
            principal: "fixture-producer".into(),
            producer: self.expected.policy.producer.clone(),
            envelope_digest: d.into(),
            validity: Validity {
                issued_at: 0,
                expires_at: 2_000_000_000,
                revoked: false,
            },
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        self.approvals
            .borrow()
            .get(reference)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
}
#[test]
fn nonnull_baseline_never_passes_without_explicit_authenticated_reference() {
    let (output, expected) = sample("clean", false, true);
    let provider = Authority::new(&expected);
    let observed = consume(&output, &expected, &provider, NOW, None);
    assert!(
        observed.is_err() || !observed.unwrap().eligibility.eligible,
        "producer authentication cannot authenticate a baseline"
    );
}

use codeguard_cli::guard_integration::consumer::{BaselineRequirement, PreparedConsumption};
fn baseline(expected: &ExpectedConsumption) -> BaselineRequirement {
    let mut policy = expected.policy.clone();
    policy.approval_principals = BTreeMap::from([(
        "baseline".into(),
        BTreeSet::from(["fixture-baseline".into()]),
    )]);
    BaselineRequirement {
        reference: "approval:baseline".into(),
        purpose: "baseline".into(),
        policy,
    }
}
fn record(expected: &ExpectedConsumption, purpose: &str, principal: &str) -> ApprovalRecord {
    ApprovalRecord {
        principal: principal.into(),
        purpose: purpose.into(),
        action: expected.policy.action.clone(),
        binding: expected.policy.binding.clone(),
        contract_digest: expected.policy.contract_digest.clone(),
        validity: Validity {
            issued_at: 0,
            expires_at: 2_000_000_000,
            revoked: false,
        },
    }
}
#[test]
fn frozen_baseline_ref_and_scope_reject_missing_extraneous_or_mismatched_contract() {
    let (_, expected) = sample("clean", false, true);
    let b = baseline(&expected);
    assert!(PreparedConsumption::freeze(&expected, None).is_err());
    let (_, without) = sample("clean", false, false);
    assert!(PreparedConsumption::freeze(&without, Some(&b)).is_err());
    for n in 0..7 {
        let mut changed = b.clone();
        match n {
            0 => changed.reference = "accepted".repeat(1025),
            1 => {
                changed.policy.binding.baseline_digest = Some(format!("sha256:{}", "f".repeat(64)))
            }
            2 => changed.policy.binding.requirement_ids = vec!["other".into()],
            3 => changed.policy.contract_digest = format!("sha256:{}", "f".repeat(64)),
            4 => changed.policy.action = "push".into(),
            5 => changed.purpose = "not-approved".into(),
            _ => changed.policy.required_scopes.clear(),
        }
        assert!(
            PreparedConsumption::freeze(&expected, Some(&changed)).is_err(),
            "{n}"
        );
    }
}
#[test]
fn baseline_queries_are_fresh_and_cannot_be_replaced_by_text_or_rehashed_issuer() {
    let (output, mut expected) = sample("clean", false, true);
    let mut b = baseline(&expected);
    let provider = Authority::new(&expected);
    let prepared = PreparedConsumption::freeze(&expected, Some(&b)).unwrap();
    assert!(
        !prepared
            .consume(&output, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    // Recomputable strings and matching envelope hashes do not create an approval record.
    provider.approvals.borrow_mut().insert(
        "accepted".into(),
        record(&expected, "baseline", "fixture-baseline"),
    );
    assert!(
        !prepared
            .consume(&output, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    let original = record(&expected, "baseline", "fixture-baseline");
    provider
        .approvals
        .borrow_mut()
        .insert(b.reference.clone(), original.clone());
    let positive = prepared.consume(&output, &provider, NOW, None).unwrap();
    assert!(positive.eligibility.eligible);
    // Mutating caller-owned expectations after freeze cannot alter the owned contract/ref.
    expected.policy.binding.baseline_digest = None;
    b.reference = "accepted".into();
    assert!(
        prepared
            .consume(&output, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    for n in 0..10 {
        let mut bad = original.clone();
        match n {
            0 => bad.principal = "forged".into(),
            1 => bad.binding.requirement_ids = vec!["other".into()],
            2 => bad.binding.baseline_digest = Some(format!("sha256:{}", "f".repeat(64))),
            3 => bad.validity.expires_at = NOW,
            4 => bad.validity.revoked = true,
            5 => bad.validity.issued_at = NOW + 1,
            6 => bad.purpose = "review".into(),
            7 => bad.action = "push".into(),
            8 => bad.contract_digest = format!("sha256:{}", "f".repeat(64)),
            _ => bad.binding.candidate_oid = "f".repeat(40),
        }
        provider
            .approvals
            .borrow_mut()
            .insert("approval:baseline".into(), bad);
        let result = prepared.consume(&output, &provider, NOW, None).unwrap();
        assert_eq!(
            result.eligibility.code,
            EligibilityCode::InvalidApproval,
            "{n}"
        );
        assert!(!result.eligibility.eligible);
        assert_ne!(result.freshness_key, positive.freshness_key);
    }
    provider.unavailable.set(true);
    assert_eq!(
        prepared
            .consume(&output, &provider, NOW, None)
            .unwrap()
            .eligibility
            .code,
        EligibilityCode::ProviderUnavailable
    );
    assert_eq!(provider.calls.get(), 15);
    assert!(prepared.consume(&output, &provider, -1, None).is_err());
}
#[test]
fn baseline_and_review_are_independent_and_original_review_bytes_do_not_change() {
    let (output, mut expected) = sample("bad", true, true);
    let original_envelope = serde_json::to_vec(output.envelope()).unwrap();
    let raw = output.domain_bytes().to_vec();
    let report = output.report_bytes().unwrap().to_vec();
    let attached = output
        .with_approval_refs(&["approval:review".into()])
        .unwrap();
    expected.envelope_digest = sha(&serde_json::to_vec(attached.envelope()).unwrap());
    let provider = Authority::new(&expected);
    let prepared = PreparedConsumption::freeze(&expected, Some(&baseline(&expected))).unwrap();
    provider.approvals.borrow_mut().insert(
        "approval:review".into(),
        record(&expected, "review", "fixture-reviewer"),
    );
    assert!(
        !prepared
            .consume_attached(&attached, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    provider.approvals.borrow_mut().insert(
        "approval:baseline".into(),
        record(&expected, "baseline", "fixture-baseline"),
    );
    let approved = prepared
        .consume_attached(&attached, &provider, NOW, None)
        .unwrap();
    assert!(approved.eligibility.eligible);
    assert_eq!(
        approved.eligibility.technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    provider
        .approvals
        .borrow_mut()
        .get_mut("approval:review")
        .unwrap()
        .validity
        .revoked = true;
    assert!(
        !prepared
            .consume_attached(&attached, &provider, NOW, None)
            .unwrap()
            .eligibility
            .eligible
    );
    assert_eq!(
        serde_json::to_vec(output.envelope()).unwrap(),
        original_envelope
    );
    assert_eq!(output.domain_bytes(), raw);
    assert_eq!(output.report_bytes().unwrap(), report);
}
#[test]
fn valid_baseline_does_not_repair_technical_block_or_partial() {
    for name in ["bad", "missing-tool"] {
        let (output, expected) = sample(name, false, true);
        let provider = Authority::new(&expected);
        provider.approvals.borrow_mut().insert(
            "approval:baseline".into(),
            record(&expected, "baseline", "fixture-baseline"),
        );
        let prepared = PreparedConsumption::freeze(&expected, Some(&baseline(&expected))).unwrap();
        let result = prepared.consume(&output, &provider, NOW, None).unwrap();
        assert!(!result.eligibility.eligible);
        assert_eq!(
            result.eligibility.technical_decision,
            Some(guardengine::Decision::Block)
        );
    }
}
