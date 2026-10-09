#[path = "guard_integration_freshness.rs"]
mod fixture;
#[test]
fn approval_attachment_is_a_separate_borrowed_envelope() {
    let (original, _) = fixture::sample(true);
    let attached = original
        .with_approval_refs(&["approval:one".into()])
        .unwrap();
    assert_eq!(attached.envelope().approval_refs, vec!["approval:one"]);
    assert!(original.envelope().approval_refs.is_empty());
    assert_eq!(original.report_bytes(), attached.report_bytes());
}

use codeguard_cli::guard_integration::consumer::{ExpectedConsumption, consume_attached};
use guardengine::integration::{GuardRunEnvelope, eligibility::*};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn expectation(original: &ExpectedConsumption, envelope: &GuardRunEnvelope) -> ExpectedConsumption {
    let mut expected = original.clone();
    expected.envelope_digest = sha(&serde_json::to_vec(envelope).unwrap());
    expected
}
// Fixed local test records; never a production signing or authority implementation.
struct ApprovalFixture {
    authenticated_envelope: RefCell<String>,
    policy: EligibilityPolicy,
    approval: RefCell<ApprovalRecord>,
    unavailable: Cell<bool>,
    producer_calls: Cell<usize>,
    approval_calls: Cell<usize>,
}
impl ApprovalFixture {
    fn new(expected: &ExpectedConsumption) -> Self {
        Self {
            authenticated_envelope: RefCell::new(expected.envelope_digest.clone()),
            policy: expected.policy.clone(),
            approval: RefCell::new(ApprovalRecord {
                principal: "fixture-reviewer".into(),
                purpose: "review".into(),
                action: expected.policy.action.clone(),
                binding: expected.policy.binding.clone(),
                contract_digest: expected.policy.contract_digest.clone(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: 2_000_000_000,
                    revoked: false,
                },
            }),
            unavailable: Cell::new(false),
            producer_calls: Cell::new(0),
            approval_calls: Cell::new(0),
        }
    }
}
impl AuthorityProvider for ApprovalFixture {
    fn verify_producer(
        &self,
        envelope: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.producer_calls.set(self.producer_calls.get() + 1);
        if digest != *self.authenticated_envelope.borrow()
            || envelope.producer != self.policy.producer
        {
            return Err(AuthorityError::Untrusted);
        }
        Ok(ProducerRecord {
            principal: "fixture-producer".into(),
            producer: self.policy.producer.clone(),
            envelope_digest: self.authenticated_envelope.borrow().clone(),
            validity: Validity {
                issued_at: 0,
                expires_at: 2_000_000_000,
                revoked: false,
            },
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.approval_calls.set(self.approval_calls.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        if reference != "approval:one" {
            return Err(AuthorityError::Untrusted);
        }
        Ok(self.approval.borrow().clone())
    }
}
#[test]
fn approved_review_requires_new_envelope_auth_and_fresh_matching_approval_every_time() {
    let (original, old_expected) = fixture::sample(true);
    let raw = original.domain_bytes().to_vec();
    let report = original.report_bytes().unwrap().to_vec();
    let attached = original
        .with_approval_refs(&["approval:one".into()])
        .unwrap();
    let expected = expectation(&old_expected, attached.envelope());
    let provider = ApprovalFixture::new(&expected);
    assert!(consume_attached(&attached, &old_expected, &provider, fixture::NOW, None).is_err());
    *provider.authenticated_envelope.borrow_mut() = old_expected.envelope_digest.clone();
    assert_eq!(
        consume_attached(&attached, &expected, &provider, fixture::NOW, None)
            .unwrap()
            .eligibility
            .code,
        EligibilityCode::UntrustedProducer
    );
    assert_eq!(provider.approval_calls.get(), 0);
    *provider.authenticated_envelope.borrow_mut() = expected.envelope_digest.clone();
    let mut key = None;
    for _ in 0..10 {
        let result = consume_attached(&attached, &expected, &provider, fixture::NOW, None).unwrap();
        assert!(result.eligibility.eligible);
        assert_eq!(
            result.eligibility.technical_decision,
            Some(guardengine::Decision::RequireApproval)
        );
        if let Some(previous) = &key {
            assert_eq!(&result.freshness_key, previous)
        } else {
            key = Some(result.freshness_key);
        }
    }
    assert_eq!(provider.approval_calls.get(), 10);
    for mutation in 0..8 {
        let baseline = provider.approval.borrow().clone();
        {
            let mut a = provider.approval.borrow_mut();
            match mutation {
                0 => a.validity.revoked = true,
                1 => a.validity.expires_at = fixture::NOW,
                2 => a.validity.issued_at = fixture::NOW + 1,
                3 => a.principal = "other".into(),
                4 => a.action = "other".into(),
                5 => a.binding.base_oid = "c".repeat(40),
                6 => a.contract_digest = format!("sha256:{}", "d".repeat(64)),
                _ => a.purpose = "other".into(),
            }
        }
        let result = consume_attached(&attached, &expected, &provider, fixture::NOW, None).unwrap();
        assert_eq!(
            result.eligibility.code,
            EligibilityCode::InvalidApproval,
            "mutation {mutation}"
        );
        assert_eq!(
            result.eligibility.technical_decision,
            Some(guardengine::Decision::RequireApproval)
        );
        assert_ne!(Some(result.freshness_key), key);
        *provider.approval.borrow_mut() = baseline;
    }
    provider.unavailable.set(true);
    assert_eq!(
        consume_attached(&attached, &expected, &provider, fixture::NOW, None)
            .unwrap()
            .eligibility
            .code,
        EligibilityCode::ProviderUnavailable
    );
    assert_eq!(provider.approval_calls.get(), 19);
    assert!(original.envelope().approval_refs.is_empty());
    assert_eq!(original.report_bytes().unwrap(), report);
    assert_eq!(original.domain_bytes(), raw);
    assert_eq!(
        attached.contract_bytes().unwrap().as_ptr(),
        original.contract_bytes().unwrap().as_ptr()
    );
    assert_eq!(
        attached.facts_bytes().unwrap().as_ptr(),
        original.facts_bytes().unwrap().as_ptr()
    );
    assert_eq!(
        attached.report_bytes().unwrap().as_ptr(),
        original.report_bytes().unwrap().as_ptr()
    );
    assert_eq!(
        attached.domain_bytes().as_ptr(),
        original.domain_bytes().as_ptr()
    );
}
#[test]
fn changed_and_removed_refs_need_independent_expected_identity_and_authentication() {
    let (original, base) = fixture::sample(true);
    let one = original
        .with_approval_refs(&["approval:one".into()])
        .unwrap();
    let one_expected = expectation(&base, one.envelope());
    let provider = ApprovalFixture::new(&one_expected);
    for refs in [vec![], vec!["approval:two".into()]] {
        let attached = original.with_approval_refs(&refs).unwrap();
        assert!(consume_attached(&attached, &one_expected, &provider, fixture::NOW, None).is_err());
        let expected = expectation(&base, attached.envelope());
        assert_eq!(
            consume_attached(&attached, &expected, &provider, fixture::NOW, None)
                .unwrap()
                .eligibility
                .code,
            EligibilityCode::UntrustedProducer
        );
        *provider.authenticated_envelope.borrow_mut() = expected.envelope_digest.clone();
        let result = consume_attached(&attached, &expected, &provider, fixture::NOW, None).unwrap();
        assert_eq!(
            result.eligibility.code,
            if refs.is_empty() {
                EligibilityCode::MissingApproval
            } else {
                EligibilityCode::InvalidApproval
            }
        );
        assert!(!result.eligibility.eligible);
        *provider.authenticated_envelope.borrow_mut() = one_expected.envelope_digest.clone();
    }
}
#[test]
fn attachment_rejects_bad_reference_sets_and_non_review_results() {
    let (review, _) = fixture::sample(true);
    for refs in [
        vec!["".into()],
        vec!["\nsecret".into()],
        vec!["a".into(), "a".into()],
        vec!["b".into(), "a".into()],
        vec!["x".repeat(1025)],
        (0..65).map(|n| format!("ref:{n:03}")).collect(),
        (0..17)
            .map(|n| format!("{n:03}{}", "x".repeat(1021)))
            .collect(),
    ] {
        assert!(review.with_approval_refs(&refs).is_err());
    }
    let escaped = review.with_approval_refs(&["\\\"".repeat(512)]).unwrap();
    assert!(serde_json::to_vec(escaped.envelope()).unwrap().len() < 1_048_576);
    let (allow, _) = fixture::sample(false);
    assert!(allow.with_approval_refs(&["approval:one".into()]).is_err());
}

#[path = "support/guard_integration.rs"]
mod native_fixture;
#[test]
fn incomplete_error_and_cancelled_outputs_cannot_receive_approval_attachments() {
    use codeguard_cli::guard_integration::{
        envelope::FrozenRun, profile::InvocationDescriptor, reader::read_native,
        scope::FrozenObligations,
    };
    let refs = vec!["approval:one".into()];
    let (partial, _) = fixture::sample_native("missing-tool", true);
    assert!(partial.with_approval_refs(&refs).is_err());
    for (exit, status) in [(4, "internal_error"), (130, "cancelled")] {
        let mut raw = native_fixture::valid();
        raw["exit_code"] = serde_json::json!(exit);
        raw["command_status"] = serde_json::json!(status);
        let mut invocation = serde_json::to_value(native_fixture::invocation()).unwrap();
        invocation["process_exit"] = serde_json::json!(exit);
        let invocation =
            InvocationDescriptor::parse(&serde_json::to_vec(&invocation).unwrap()).unwrap();
        let evidence = read_native(&serde_json::to_vec(&raw).unwrap(), &invocation).unwrap();
        let (_, expected) = fixture::sample(false);
        let mut binding = expected.policy.binding;
        binding.source_snapshot_digest = format!(
            "sha256:{}",
            raw["identities"]["content"]["digest"].as_str().unwrap()
        );
        let scope = FrozenObligations::new(std::collections::BTreeMap::from([(
            "python/app/lint/ruff".into(),
            vec!["src/main.py".into()],
        )]))
        .unwrap();
        let output = FrozenRun::new(
            raw["run_id"].as_str().unwrap(),
            binding,
            &scope,
            "2026-10-09T00:00:00Z",
            "2026-10-09T00:01:00Z",
        )
        .unwrap()
        .failure(&evidence)
        .unwrap();
        assert!(output.with_approval_refs(&refs).is_err());
        assert!(output.envelope().decision.is_none());
        assert!(output.report_bytes().is_none());
    }
}

#[test]
fn escaped_reference_serialization_budget_is_checked_before_new_envelope_clone() {
    let requirements = (0..1000)
        .map(|n| format!("{n:04}{}", "x".repeat(1020)))
        .collect();
    let (review, _) = fixture::sample_with_requirements("bad", true, requirements);
    let refs: Vec<String> = (0..16)
        .map(|n| format!("{n:04}{}", "\"".repeat(1020)))
        .collect();
    assert_eq!(refs.iter().map(String::len).sum::<usize>(), 16_384);
    let original_size = serde_json::to_vec(review.envelope()).unwrap().len();
    assert!(original_size <= 1_048_576);
    assert!(original_size - 2 + serde_json::to_vec(&refs).unwrap().len() > 1_048_576);
    assert_eq!(
        review.with_approval_refs(&refs).err().unwrap(),
        "approval envelope budget exceeded"
    );
    assert!(review.envelope().approval_refs.is_empty());
}
