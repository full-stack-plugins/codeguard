use codeguard_cli::false_positive_decision::parse_false_positive_decision_candidate;
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, bind_signed_false_positive_preview,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::json;
use sha2::{Digest, Sha256};

struct Fixture {
    candidate: Vec<u8>,
    snapshot: Vec<u8>,
    envelope: Vec<u8>,
    trust: ApprovalTrustKey,
}
fn context() -> ApprovalVerificationContext<'static> {
    ApprovalVerificationContext {
        workspace_id: "workspace-one",
        policy_revision: "policy-r1",
        baseline_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        now_unix: Some(150),
        minimum_sequence: 1,
        max_lifetime_seconds: 100,
    }
}
fn fixture(candidate_expiry: u64) -> Fixture {
    let candidate = serde_json::to_vec(&json!({"schema_version":"1.0","kind":"false_positive","id":"FP-1","identity":{"finding_id":"CG-1","checker_id":"python.ruff","native_rule_id":"F401","category":"lint","target":{"kind":"source","path":"app.py","file_sha256":"a".repeat(64)},"finding_fingerprint":"b".repeat(64),"tool_sha256":"c".repeat(64),"adapter_sha256":"d".repeat(64),"rulepack_sha256":"e".repeat(64)},"reason_code":"native_false_positive","rationale":"误报复核","reproducer_ref":"case-1","approved_policy_revision":"policy-r1","approval_ref":"review-1","reviewer":"reviewer-1","created_at":100,"expires_at":candidate_expiry})).unwrap();
    let snapshot = serde_json::to_vec(&json!({"schema_version":"1.0","policy_revision":"policy-r1","max_lifetime_seconds":1000,"decisions":[{"id":"FP-1","sha256":format!("{:x}",Sha256::digest(&candidate))}]})).unwrap();
    let payload = serde_json::to_string(&json!({"schema_version":"1.0","workspace_id":"workspace-one","policy_revision":"policy-r1","baseline_commit":"a".repeat(40),"revision_sequence":1,"issued_at":100,"expires_at":200,"snapshot_sha256":format!("{:x}",Sha256::digest(&snapshot))})).unwrap();
    let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let mut message = b"codeguard.approval.v1\0review-key\0".to_vec();
    message.extend_from_slice(payload.as_bytes());
    let signature: String = pair
        .sign(&message)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let envelope = serde_json::to_vec(&json!({"schema_version":"1.0","key_id":"review-key","approval_json":payload,"signature_hex":signature})).unwrap();
    Fixture {
        candidate,
        snapshot,
        envelope,
        trust: ApprovalTrustKey {
            key_id: "review-key".into(),
            public_key: pair.public_key().as_ref().try_into().unwrap(),
            valid_from: 1,
            valid_until: 1000,
            revoked: false,
        },
    }
}

#[test]
fn signed_candidate_binding_preserves_exact_metadata_without_granting_authority() {
    for (candidate_expiry, effective_expiry) in [(200, 200), (180, 180)] {
        let f = fixture(candidate_expiry);
        let identity = parse_false_positive_decision_candidate(&f.candidate)
            .unwrap()
            .identity;
        let bound = bind_signed_false_positive_preview(
            &f.candidate,
            &identity,
            &f.snapshot,
            &f.envelope,
            &f.trust,
            &context(),
        )
        .unwrap();
        let preview = bound.preview();
        assert_eq!(preview.observed_identity, identity);
        assert_eq!(preview.decision_identity, identity);
        assert_eq!(preview.decision_id, "FP-1");
        assert_eq!(preview.approval_ref, "review-1");
        assert_eq!(preview.approved_policy_revision, "policy-r1");
        assert_eq!(preview.observed_at, 150);
        assert_eq!(preview.expires_at, effective_expiry);
        assert!(!preview.independent_approval_verified);
        assert_eq!(
            bound.candidate_sha256(),
            format!("{:x}", Sha256::digest(&f.candidate))
        );
        assert_eq!(
            bound.snapshot_sha256(),
            format!("{:x}", Sha256::digest(&f.snapshot))
        );
        assert_eq!(bound.workspace_id(), "workspace-one");
        assert_eq!(bound.baseline_commit(), context().baseline_commit);
        assert_eq!(
            preview.approval_scope.as_ref().unwrap().workspace_id,
            bound.workspace_id()
        );
        assert_eq!(
            preview.approval_scope.as_ref().unwrap().baseline_commit,
            bound.baseline_commit()
        );
        assert_eq!(bound.revision_sequence(), 1);
        assert_eq!(bound.signing_key_id(), "review-key");
    }
}

#[test]
fn candidate_or_observation_drift_and_expired_or_revoked_signature_cannot_make_a_bound_preview() {
    let f = fixture(200);
    let identity = parse_false_positive_decision_candidate(&f.candidate)
        .unwrap()
        .identity;
    let mut changed = identity.clone();
    changed.native_rule_id = "F402".into();
    assert!(
        bind_signed_false_positive_preview(
            &f.candidate,
            &changed,
            &f.snapshot,
            &f.envelope,
            &f.trust,
            &context()
        )
        .is_err()
    );
    let mut bytes = f.candidate.clone();
    bytes.push(b' ');
    assert!(
        bind_signed_false_positive_preview(
            &bytes,
            &identity,
            &f.snapshot,
            &f.envelope,
            &f.trust,
            &context()
        )
        .is_err()
    );
    let mut host = context();
    host.now_unix = Some(200);
    assert!(
        bind_signed_false_positive_preview(
            &f.candidate,
            &identity,
            &f.snapshot,
            &f.envelope,
            &f.trust,
            &host
        )
        .is_err()
    );
    let mut key = f.trust.clone();
    key.revoked = true;
    assert!(
        bind_signed_false_positive_preview(
            &f.candidate,
            &identity,
            &f.snapshot,
            &f.envelope,
            &key,
            &context()
        )
        .is_err()
    );
    assert!(
        bind_signed_false_positive_preview(
            &vec![b' '; 64 * 1024 + 1],
            &identity,
            &f.snapshot,
            &f.envelope,
            &f.trust,
            &context()
        )
        .is_err()
    );
}

#[test]
fn bound_preview_never_auto_approves_and_final_gate_honors_signed_expiry_and_policy() {
    use codeguard_core::{
        Completion, DeliveryDecision, DeliveryInput, Finding, FindingLocation, GateImpact,
        NativeCheckerBinding, ObligationEvidence, ObligationResult, ObligationSpec,
        evaluate_delivery,
    };
    let f = fixture(200);
    let identity = parse_false_positive_decision_candidate(&f.candidate)
        .unwrap()
        .identity;
    let bound = bind_signed_false_positive_preview(
        &f.candidate,
        &identity,
        &f.snapshot,
        &f.envelope,
        &f.trust,
        &context(),
    )
    .unwrap();
    let mut input = DeliveryInput {
        approval_scope: Some(codeguard_core::ApprovalScope {
            workspace_id: "workspace-one".into(),
            baseline_commit: "a".repeat(40),
        }),
        full_project: true,
        discovery_complete: true,
        trusted_bindings_verified: true,
        gate_time_unix: Some(150),
        policy_revision: Some("policy-r1".into()),
        frozen_obligation_ids: vec!["python/lint".into()],
        obligations: vec![ObligationSpec {
            id: "python/lint".into(),
            expected_targets: vec!["app.py".into()],
        }],
        evidence: vec![ObligationEvidence {
            observed_targets: vec!["app.py".into()],
            result: ObligationResult {
                id: "python/lint".into(),
                completion: Completion::Complete,
                reason: None,
                findings: vec![Finding {
                    id: "CG-1".into(),
                    native_rule_id: "F401".into(),
                    tool_id: "ruff".into(),
                    severity: "error".into(),
                    gate_impact: GateImpact::Blocking,
                    message: "fixture".into(),
                    obligation_id: "python/lint".into(),
                    native_identity: Some(identity.clone()),
                    locations: vec![FindingLocation::Source {
                        path: "app.py".into(),
                        line: Some(1),
                        column: None,
                    }],
                }],
            },
        }],
        native_checker_bindings: vec![NativeCheckerBinding {
            checker_id: "python.ruff".into(),
            tool_id: "ruff".into(),
            category: "lint".into(),
            obligation_id: "python/lint".into(),
        }],
        allowlist_dispositions: vec![bound.preview().clone()],
    };
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    // 仅在合成模型中模拟独立来源审核；产品没有由此签发批准的接口。
    input.allowlist_dispositions[0].independent_approval_verified = true;
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::AllowWithExceptions
    );
    input.gate_time_unix = Some(200);
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
    input.gate_time_unix = Some(150);
    input.policy_revision = Some("policy-r2".into());
    assert_eq!(
        evaluate_delivery(&input).decision,
        DeliveryDecision::Incomplete
    );
}

#[test]
fn signed_candidate_outside_signature_is_rejected_instead_of_clamped() {
    let f = fixture(300);
    let identity = parse_false_positive_decision_candidate(&f.candidate)
        .unwrap()
        .identity;
    let result = bind_signed_false_positive_preview(
        &f.candidate,
        &identity,
        &f.snapshot,
        &f.envelope,
        &f.trust,
        &context(),
    );
    assert_eq!(
        result.err().unwrap(),
        "approval_candidate_lifetime_outside_signature"
    );
}
