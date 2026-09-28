use codeguard_cli::{ApprovalTrustKey, ApprovalVerificationContext, verify_signed_approval};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn key(seed: u8) -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap()
}
fn payload(snapshot: &[u8]) -> Value {
    json!({"schema_version":"1.0","workspace_id":"workspace-one",
        "policy_revision":"policy-r2","baseline_commit":"a".repeat(40),
        "revision_sequence":2,"issued_at":100,"expires_at":200,
        "snapshot_sha256":format!("{:x}",Sha256::digest(snapshot))})
}
fn envelope(pair: &Ed25519KeyPair, value: &Value) -> Vec<u8> {
    sign_raw(pair, &serde_json::to_string(value).unwrap())
}
fn sign_raw(pair: &Ed25519KeyPair, raw: &str) -> Vec<u8> {
    let mut message = b"codeguard.approval.v1\0review-key\0".to_vec();
    message.extend_from_slice(raw.as_bytes());
    let signature = pair.sign(&message);
    let hex: String = signature
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    serde_json::to_vec(&json!({"schema_version":"1.0","key_id":"review-key","approval_json":raw,"signature_hex":hex})).unwrap()
}
fn anchor(pair: &Ed25519KeyPair) -> ApprovalTrustKey {
    ApprovalTrustKey {
        key_id: "review-key".into(),
        public_key: pair.public_key().as_ref().try_into().unwrap(),
        valid_from: 1,
        valid_until: 300,
        revoked: false,
    }
}
fn context() -> ApprovalVerificationContext<'static> {
    ApprovalVerificationContext {
        workspace_id: "workspace-one",
        policy_revision: "policy-r2",
        baseline_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        now_unix: Some(150),
        minimum_sequence: 2,
        max_lifetime_seconds: 100,
    }
}

#[test]
fn zero_oid_is_not_a_protected_code_baseline_even_with_a_valid_signature() {
    let pair = key(7);
    let trust = anchor(&pair);
    let snapshot = b"fixture snapshot";
    for length in [40, 64] {
        let zero = "0".repeat(length);
        let mut value = payload(snapshot);
        value["baseline_commit"] = json!(zero);
        let bytes = envelope(&pair, &value);
        let mut host = context();
        host.baseline_commit = &zero;
        assert!(matches!(
            verify_signed_approval(&bytes, snapshot, &trust, &host),
            Err("approval_context_invalid")
        ));
    }
}

#[test]
fn signed_approval_binds_exact_bytes_scope_and_host_context() {
    let pair = key(7);
    let snapshot = b"exact approval snapshot";
    let value = payload(snapshot);
    let signed = envelope(&pair, &value);
    let trusted = anchor(&pair);
    let verified = verify_signed_approval(&signed, snapshot, &trusted, &context()).unwrap();
    assert_eq!(verified.policy_revision(), "policy-r2");
    assert_eq!(verified.revision_sequence(), 2);
    assert_eq!(verified.signing_key_id(), "review-key");
    assert_eq!(verified.expires_at(), 200);
    assert_eq!(
        verified.snapshot_sha256(),
        format!("{:x}", Sha256::digest(snapshot))
    );
    assert!(verify_signed_approval(&signed, b"changed", &trusted, &context()).is_err());
    let mut changed_envelope: Value = serde_json::from_slice(&signed).unwrap();
    let original_json = changed_envelope["approval_json"]
        .as_str()
        .unwrap()
        .to_owned();
    changed_envelope["approval_json"] = json!(format!("{original_json} "));
    assert!(
        verify_signed_approval(
            &serde_json::to_vec(&changed_envelope).unwrap(),
            snapshot,
            &trusted,
            &context()
        )
        .is_err()
    );
    let wrong_domain = pair.sign(original_json.as_bytes());
    changed_envelope["approval_json"] = json!(original_json);
    changed_envelope["signature_hex"] = json!(
        wrong_domain
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert!(
        verify_signed_approval(
            &serde_json::to_vec(&changed_envelope).unwrap(),
            snapshot,
            &trusted,
            &context()
        )
        .is_err()
    );
    assert!(
        verify_signed_approval(&envelope(&key(8), &value), snapshot, &trusted, &context()).is_err()
    );
    for field in ["workspace_id", "policy_revision", "baseline_commit"] {
        let mut changed = value.clone();
        changed[field] = json!("other");
        assert!(
            verify_signed_approval(&envelope(&pair, &changed), snapshot, &trusted, &context())
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn expired_future_revoked_unknown_clock_and_rollback_never_authorize() {
    let pair = key(7);
    let snapshot = b"snapshot";
    let value = payload(snapshot);
    let signed = envelope(&pair, &value);
    let trusted = anchor(&pair);
    for now in [None, Some(99), Some(200)] {
        let mut ctx = context();
        ctx.now_unix = now;
        assert!(verify_signed_approval(&signed, snapshot, &trusted, &ctx).is_err());
    }
    for kind in [
        "revoked",
        "key_expired",
        "key_future",
        "key_id",
        "rollback",
        "lifetime",
    ] {
        let mut trust = anchor(&pair);
        let mut ctx = context();
        match kind {
            "revoked" => trust.revoked = true,
            "key_expired" => trust.valid_until = 150,
            "key_future" => trust.valid_from = 151,
            "key_id" => trust.key_id = "other".into(),
            "rollback" => ctx.minimum_sequence = 3,
            _ => ctx.max_lifetime_seconds = 99,
        }
        assert!(
            verify_signed_approval(&signed, snapshot, &trust, &ctx).is_err(),
            "{kind}"
        );
    }
}

#[test]
fn malformed_or_ambiguous_signed_documents_are_rejected() {
    let pair = key(7);
    let snapshot = b"snapshot";
    let value = payload(snapshot);
    let trusted = anchor(&pair);
    let raw = serde_json::to_string(&value).unwrap();
    let duplicate = raw.replacen('{', "{\"workspace_id\":\"workspace-one\",", 1);
    assert!(
        verify_signed_approval(&sign_raw(&pair, &duplicate), snapshot, &trusted, &context())
            .is_err()
    );
    for field in [
        "unknown",
        "schema_version",
        "expires_at",
        "revision_sequence",
    ] {
        let mut changed = value.clone();
        changed[field] = match field {
            "schema_version" => json!("2.0"),
            "expires_at" => json!(100),
            "revision_sequence" => json!(0),
            _ => json!(true),
        };
        assert!(
            verify_signed_approval(&envelope(&pair, &changed), snapshot, &trusted, &context())
                .is_err()
        );
    }
    let mut signed: Value = serde_json::from_slice(&envelope(&pair, &value)).unwrap();
    signed["approved"] = json!(true);
    assert!(
        verify_signed_approval(
            &serde_json::to_vec(&signed).unwrap(),
            snapshot,
            &trusted,
            &context()
        )
        .is_err()
    );
    signed.as_object_mut().unwrap().remove("approved");
    signed["signature_hex"] = json!("0".repeat(128));
    assert!(
        verify_signed_approval(
            &serde_json::to_vec(&signed).unwrap(),
            snapshot,
            &trusted,
            &context()
        )
        .is_err()
    );
    signed["signature_hex"] = json!("00");
    assert!(
        verify_signed_approval(
            &serde_json::to_vec(&signed).unwrap(),
            snapshot,
            &trusted,
            &context()
        )
        .is_err()
    );
    assert!(verify_signed_approval(&vec![b' '; 16385], snapshot, &trusted, &context()).is_err());
}

#[test]
fn signed_snapshot_still_requires_exact_candidate_revision_and_bytes() {
    use codeguard_cli::approval_snapshot::{SnapshotResolution, verify_and_bind_candidate};
    use codeguard_cli::false_positive_decision::parse_false_positive_decision_candidate;
    let pair = key(7);
    let trusted = anchor(&pair);
    let candidate = json!({"schema_version":"1.0","kind":"false_positive","id":"FP-1",
        "identity":{"finding_id":"CG-1","checker_id":"ruff","native_rule_id":"F401","category":"lint",
            "target":{"kind":"source","path":"app.py","file_sha256":"a".repeat(64)},
            "finding_fingerprint":"b".repeat(64),"tool_sha256":"c".repeat(64),"adapter_sha256":"d".repeat(64),"rulepack_sha256":"e".repeat(64)},
        "reason_code":"native_false_positive","rationale":"复核","reproducer_ref":"case-1",
        "approved_policy_revision":"policy-r2","approval_ref":"review-1","reviewer":"reviewer-1","created_at":100,"expires_at":200});
    let bytes = serde_json::to_vec(&candidate).unwrap();
    let observed = parse_false_positive_decision_candidate(&bytes)
        .unwrap()
        .identity;
    let mut snapshot = json!({"schema_version":"1.0","policy_revision":"policy-r2","max_lifetime_seconds":100,
        "decisions":[{"id":"FP-1","sha256":format!("{:x}",Sha256::digest(&bytes))}]});
    let raw = serde_json::to_vec(&snapshot).unwrap();
    let signed = envelope(&pair, &payload(&raw));
    assert_eq!(
        verify_and_bind_candidate(&bytes, &observed, &raw, &signed, &trusted, &context()).unwrap(),
        SnapshotResolution::BoundToPinnedSnapshot
    );
    let mut changed = candidate.clone();
    changed["rationale"] = json!("自行修改");
    assert_eq!(
        verify_and_bind_candidate(
            &serde_json::to_vec(&changed).unwrap(),
            &observed,
            &raw,
            &signed,
            &trusted,
            &context()
        )
        .unwrap(),
        SnapshotResolution::AbsentFromSnapshot
    );
    snapshot["policy_revision"] = json!("policy-r3");
    let wrong = serde_json::to_vec(&snapshot).unwrap();
    let signed = envelope(&pair, &payload(&wrong));
    assert_eq!(
        verify_and_bind_candidate(&bytes, &observed, &wrong, &signed, &trusted, &context()),
        Err("approval_snapshot_revision_mismatch")
    );
}
