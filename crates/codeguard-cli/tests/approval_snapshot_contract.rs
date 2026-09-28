use codeguard_cli::approval_snapshot::{
    PriorApprovalInput, SnapshotResolution, bind_candidate_to_snapshot, bind_replacement_chain,
    bind_replacement_to_snapshot, bind_replacement_with_prior_snapshot,
};
use codeguard_cli::false_positive_decision::parse_false_positive_decision_candidate;
use codeguard_core::FalsePositiveIdentity;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn candidate() -> Value {
    json!({
        "schema_version":"1.0", "kind":"false_positive", "id":"CG-FP-1",
        "identity":{
            "finding_id":"CG-F401-1", "checker_id":"ruff", "native_rule_id":"F401",
            "category":"lint", "target":{"kind":"source","path":"src/app.py","file_sha256":"a".repeat(64)},
            "finding_fingerprint":"b".repeat(64), "tool_sha256":"c".repeat(64),
            "adapter_sha256":"d".repeat(64), "rulepack_sha256":"e".repeat(64)
        },
        "reason_code":"native_false_positive", "rationale":"原生误报的复现依据",
        "reproducer_ref":"evidence:case-1", "approved_policy_revision":"policy-r42",
        "approval_ref":"review:123", "reviewer":"reviewer-1",
        "created_at":1790000000_u64, "expires_at":1791000000_u64
    })
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn observed() -> FalsePositiveIdentity {
    parse_false_positive_decision_candidate(&serde_json::to_vec(&candidate()).unwrap())
        .unwrap()
        .identity
}

fn snapshot(candidate_bytes: &[u8]) -> Value {
    json!({
        "schema_version":"1.0", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-1", "sha256":sha(candidate_bytes)}]
    })
}

fn bind(candidate_bytes: &[u8], snapshot: &Value, pin: Option<&str>) -> SnapshotResolution {
    let snapshot_bytes = serde_json::to_vec(snapshot).unwrap();
    let actual = sha(&snapshot_bytes);
    bind_candidate_to_snapshot(
        candidate_bytes,
        &observed(),
        &snapshot_bytes,
        pin.or(Some(&actual)),
        Some(1790500000),
    )
}

#[test]
fn exact_prior_snapshot_binding_is_only_a_binding_not_delivery_authority() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    assert_eq!(
        bind(&bytes, &snapshot(&bytes), None),
        SnapshotResolution::BoundToPinnedSnapshot
    );
}

#[test]
fn candidate_added_after_snapshot_is_not_authorized_by_approval_text() {
    let prior = serde_json::to_vec(&candidate()).unwrap();
    let mut changed = candidate();
    changed["approval_ref"] = json!("review:self-approved");
    let changed = serde_json::to_vec(&changed).unwrap();
    assert_eq!(
        bind(&changed, &snapshot(&prior), None),
        SnapshotResolution::AbsentFromSnapshot
    );
}

#[test]
fn changed_snapshot_or_missing_pin_fails_closed() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let snapshot = snapshot(&bytes);
    assert_eq!(
        bind(&bytes, &snapshot, Some(&"0".repeat(64))),
        SnapshotResolution::PinMismatch
    );
    let snapshot_bytes = serde_json::to_vec(&snapshot).unwrap();
    assert_eq!(
        bind_candidate_to_snapshot(&bytes, &observed(), &snapshot_bytes, None, Some(1790500000)),
        SnapshotResolution::Unpinned
    );
}

#[test]
fn duplicate_or_malformed_snapshot_is_rejected() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let mut duplicated = snapshot(&bytes);
    let row = duplicated["decisions"][0].clone();
    duplicated["decisions"].as_array_mut().unwrap().push(row);
    assert_eq!(
        bind(&bytes, &duplicated, None),
        SnapshotResolution::InvalidSnapshot
    );
    let mut malformed = snapshot(&bytes);
    malformed["approved"] = json!(true);
    assert_eq!(
        bind(&bytes, &malformed, None),
        SnapshotResolution::InvalidSnapshot
    );
    let mut broad = snapshot(&bytes);
    broad["policy_revision"] = json!("policy-*");
    assert_eq!(
        bind(&bytes, &broad, None),
        SnapshotResolution::InvalidSnapshot
    );
}

#[test]
fn expired_or_unreliable_clock_cannot_bind() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let snapshot = snapshot(&bytes);
    let snapshot_bytes = serde_json::to_vec(&snapshot).unwrap();
    let pin = sha(&snapshot_bytes);
    assert_eq!(
        bind_candidate_to_snapshot(&bytes, &observed(), &snapshot_bytes, Some(&pin), None),
        SnapshotResolution::UnverifiableContext
    );
    assert_eq!(
        bind_candidate_to_snapshot(
            &bytes,
            &observed(),
            &snapshot_bytes,
            Some(&pin),
            Some(1791000000)
        ),
        SnapshotResolution::Expired
    );
}

#[test]
fn pinned_candidate_for_a_different_native_finding_does_not_match() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let snapshot_bytes = serde_json::to_vec(&snapshot(&bytes)).unwrap();
    let pin = sha(&snapshot_bytes);
    let mut actual = observed();
    actual.native_rule_id = "F402".into();
    assert_eq!(
        bind_candidate_to_snapshot(
            &bytes,
            &actual,
            &snapshot_bytes,
            Some(&pin),
            Some(1790500000)
        ),
        SnapshotResolution::ObservationMismatch
    );
}

#[test]
fn revoked_decision_cannot_bind_even_when_its_bytes_remain_pinned() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let mut revised = snapshot(&bytes);
    revised["schema_version"] = json!("1.1");
    revised["revoked_decision_ids"] = json!(["CG-FP-1"]);
    assert_eq!(bind(&bytes, &revised, None), SnapshotResolution::Revoked);
}

#[test]
fn revised_snapshot_accepts_new_decision_and_rejects_old_one() {
    let old_bytes = serde_json::to_vec(&candidate()).unwrap();
    let mut new_candidate = candidate();
    new_candidate["id"] = json!("CG-FP-2");
    new_candidate["schema_version"] = json!("1.1");
    new_candidate["replaces_decision_id"] = json!("CG-FP-1");
    let new_bytes = serde_json::to_vec(&new_candidate).unwrap();
    let revised = json!({
        "schema_version":"1.1", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-2", "sha256":sha(&new_bytes)}],
        "revoked_decision_ids":["CG-FP-1"]
    });
    assert_eq!(
        bind(&old_bytes, &revised, None),
        SnapshotResolution::Revoked
    );
    assert_eq!(
        bind(&new_bytes, &revised, None),
        SnapshotResolution::ReplacementPriorIdentityUnverified
    );
    let revised_bytes = serde_json::to_vec(&revised).unwrap();
    let pin = sha(&revised_bytes);
    assert_eq!(
        bind_replacement_to_snapshot(
            &new_bytes,
            &old_bytes,
            &observed(),
            &revised_bytes,
            Some(&pin),
            Some(1790500000)
        ),
        SnapshotResolution::BoundToPinnedSnapshot
    );
}

#[test]
fn replacement_cannot_bind_unless_prior_decision_is_atomically_revoked() {
    let mut replacement = candidate();
    replacement["schema_version"] = json!("1.1");
    replacement["id"] = json!("CG-FP-2");
    replacement["replaces_decision_id"] = json!("CG-FP-1");
    let bytes = serde_json::to_vec(&replacement).unwrap();
    let mut revised = snapshot(&bytes);
    revised["schema_version"] = json!("1.1");
    revised["decisions"][0]["id"] = json!("CG-FP-2");
    revised["revoked_decision_ids"] = json!([]);
    assert_eq!(
        bind(&bytes, &revised, None),
        SnapshotResolution::ReplacementPredecessorNotRevoked
    );
    revised["revoked_decision_ids"] = json!(["CG-FP-1"]);
    assert_eq!(
        bind(&bytes, &revised, None),
        SnapshotResolution::ReplacementPriorIdentityUnverified
    );
}

#[test]
fn replacement_rejects_different_finding_or_target_from_prior_decision() {
    let old_bytes = serde_json::to_vec(&candidate()).unwrap();
    let mut replacement = candidate();
    replacement["schema_version"] = json!("1.1");
    replacement["id"] = json!("CG-FP-2");
    replacement["replaces_decision_id"] = json!("CG-FP-1");
    let revised = json!({
        "schema_version":"1.1", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-2", "sha256":sha(&serde_json::to_vec(&replacement).unwrap())}],
        "revoked_decision_ids":["CG-FP-1"]
    });
    let revised_bytes = serde_json::to_vec(&revised).unwrap();
    let pin = sha(&revised_bytes);
    replacement["identity"]["finding_id"] = json!("CG-OTHER");
    let changed = serde_json::to_vec(&replacement).unwrap();
    assert_eq!(
        bind_replacement_to_snapshot(
            &changed,
            &old_bytes,
            &observed(),
            &revised_bytes,
            Some(&pin),
            Some(1790500000)
        ),
        SnapshotResolution::ReplacementPriorIdentityMismatch
    );
    replacement["identity"]["finding_id"] = json!("CG-F401-1");
    replacement["identity"]["target"]["path"] = json!("src/other.py");
    let changed = serde_json::to_vec(&replacement).unwrap();
    assert_eq!(
        bind_replacement_to_snapshot(
            &changed,
            &old_bytes,
            &observed(),
            &revised_bytes,
            Some(&pin),
            Some(1790500000)
        ),
        SnapshotResolution::ReplacementPriorIdentityMismatch
    );
}

#[test]
fn replacement_binding_requires_an_actual_replacement_candidate() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let snapshot = snapshot(&bytes);
    let snapshot_bytes = serde_json::to_vec(&snapshot).unwrap();
    let pin = sha(&snapshot_bytes);
    assert_eq!(
        bind_replacement_to_snapshot(
            &bytes,
            &bytes,
            &observed(),
            &snapshot_bytes,
            Some(&pin),
            Some(1790500000)
        ),
        SnapshotResolution::ReplacementCandidateRequired
    );
}

#[test]
fn replacement_chain_requires_a_pinned_prior_snapshot_containing_the_old_decision() {
    let mut prior = candidate();
    prior["approved_policy_revision"] = json!("policy-r41");
    let prior_bytes = serde_json::to_vec(&prior).unwrap();
    let prior_snapshot = json!({
        "schema_version":"1.0", "policy_revision":"policy-r41",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-1","sha256":sha(&prior_bytes)}]
    });
    let prior_snapshot_bytes = serde_json::to_vec(&prior_snapshot).unwrap();
    let prior_pin = sha(&prior_snapshot_bytes);
    let mut replacement = candidate();
    replacement["schema_version"] = json!("1.1");
    replacement["id"] = json!("CG-FP-2");
    replacement["replaces_decision_id"] = json!("CG-FP-1");
    let replacement_bytes = serde_json::to_vec(&replacement).unwrap();
    let current_snapshot = json!({
        "schema_version":"1.1", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-2","sha256":sha(&replacement_bytes)}],
        "revoked_decision_ids":["CG-FP-1"]
    });
    let current_bytes = serde_json::to_vec(&current_snapshot).unwrap();
    let current_pin = sha(&current_bytes);
    let bind = |old_bytes: &[u8], old_snapshot: &[u8], old_pin: Option<&str>| {
        bind_replacement_with_prior_snapshot(
            &replacement_bytes,
            &observed(),
            &current_bytes,
            Some(&current_pin),
            PriorApprovalInput {
                decision_bytes: old_bytes,
                snapshot_bytes: old_snapshot,
                expected_snapshot_sha256: old_pin,
            },
            Some(1790500000),
        )
    };
    assert_eq!(
        bind(&prior_bytes, &prior_snapshot_bytes, Some(&prior_pin)),
        SnapshotResolution::BoundToPinnedSnapshot
    );
    assert_eq!(
        bind(&prior_bytes, &prior_snapshot_bytes, None),
        SnapshotResolution::PriorSnapshotUnpinned
    );
    assert_eq!(
        bind(&prior_bytes, &prior_snapshot_bytes, Some(&"0".repeat(64))),
        SnapshotResolution::PriorSnapshotPinMismatch
    );
    let mut altered = prior.clone();
    altered["approval_ref"] = json!("review:forged");
    assert_eq!(
        bind(
            &serde_json::to_vec(&altered).unwrap(),
            &prior_snapshot_bytes,
            Some(&prior_pin)
        ),
        SnapshotResolution::PriorDecisionAbsentFromSnapshot
    );
    let mut revoked = prior_snapshot.clone();
    revoked["schema_version"] = json!("1.1");
    revoked["revoked_decision_ids"] = json!(["CG-FP-1"]);
    let revoked_bytes = serde_json::to_vec(&revoked).unwrap();
    assert_eq!(
        bind(&prior_bytes, &revoked_bytes, Some(&sha(&revoked_bytes))),
        SnapshotResolution::PriorDecisionRevoked
    );
    let mut overlong = prior_snapshot.clone();
    overlong["max_lifetime_seconds"] = json!(1);
    let overlong_bytes = serde_json::to_vec(&overlong).unwrap();
    assert_eq!(
        bind(&prior_bytes, &overlong_bytes, Some(&sha(&overlong_bytes))),
        SnapshotResolution::PriorDecisionLifetimeExceeded
    );
    let mut same_revision = prior.clone();
    same_revision["approved_policy_revision"] = json!("policy-r42");
    let same_revision_bytes = serde_json::to_vec(&same_revision).unwrap();
    let same_snapshot = json!({
        "schema_version":"1.0", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-1","sha256":sha(&same_revision_bytes)}]
    });
    let same_snapshot_bytes = serde_json::to_vec(&same_snapshot).unwrap();
    assert_eq!(
        bind(
            &same_revision_bytes,
            &same_snapshot_bytes,
            Some(&sha(&same_snapshot_bytes))
        ),
        SnapshotResolution::PriorPolicyRevisionConflict
    );
}

#[test]
fn complete_multihop_replacement_chain_rejects_truncation_extra_hops_and_cycle() {
    let mut root = candidate();
    root["approved_policy_revision"] = json!("policy-r40");
    let root_bytes = serde_json::to_vec(&root).unwrap();
    let root_snapshot = json!({
        "schema_version":"1.0", "policy_revision":"policy-r40",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-1","sha256":sha(&root_bytes)}]
    });
    let root_snapshot_bytes = serde_json::to_vec(&root_snapshot).unwrap();
    let root_pin = sha(&root_snapshot_bytes);

    let mut middle = candidate();
    middle["schema_version"] = json!("1.1");
    middle["id"] = json!("CG-FP-2");
    middle["replaces_decision_id"] = json!("CG-FP-1");
    middle["approved_policy_revision"] = json!("policy-r41");
    let middle_bytes = serde_json::to_vec(&middle).unwrap();
    let middle_snapshot = json!({
        "schema_version":"1.1", "policy_revision":"policy-r41",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-2","sha256":sha(&middle_bytes)}],
        "revoked_decision_ids":["CG-FP-1"]
    });
    let middle_snapshot_bytes = serde_json::to_vec(&middle_snapshot).unwrap();
    let middle_pin = sha(&middle_snapshot_bytes);

    let mut current = candidate();
    current["schema_version"] = json!("1.1");
    current["id"] = json!("CG-FP-3");
    current["replaces_decision_id"] = json!("CG-FP-2");
    let current_bytes = serde_json::to_vec(&current).unwrap();
    let current_snapshot = json!({
        "schema_version":"1.1", "policy_revision":"policy-r42",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-3","sha256":sha(&current_bytes)}],
        "revoked_decision_ids":["CG-FP-2"]
    });
    let current_snapshot_bytes = serde_json::to_vec(&current_snapshot).unwrap();
    let current_pin = sha(&current_snapshot_bytes);
    let middle_input = PriorApprovalInput {
        decision_bytes: &middle_bytes,
        snapshot_bytes: &middle_snapshot_bytes,
        expected_snapshot_sha256: Some(&middle_pin),
    };
    let root_input = PriorApprovalInput {
        decision_bytes: &root_bytes,
        snapshot_bytes: &root_snapshot_bytes,
        expected_snapshot_sha256: Some(&root_pin),
    };
    let check = |priors: &[PriorApprovalInput<'_>]| {
        bind_replacement_chain(
            &current_bytes,
            &observed(),
            &current_snapshot_bytes,
            Some(&current_pin),
            priors,
            Some(1790500000),
        )
    };
    assert_eq!(
        check(&[middle_input, root_input]),
        SnapshotResolution::BoundToPinnedSnapshot
    );
    assert_eq!(
        check(&[middle_input]),
        SnapshotResolution::RevisionChainTruncated
    );
    assert_eq!(check(&[]), SnapshotResolution::RevisionChainTruncated);
    assert_eq!(
        check(&[root_input, middle_input]),
        SnapshotResolution::ReplacementPriorIdentityMismatch
    );
    assert_eq!(
        check(&[middle_input, root_input, root_input]),
        SnapshotResolution::RevisionChainUnexpectedExtra
    );
    assert_eq!(
        check(&vec![root_input; 33]),
        SnapshotResolution::RevisionChainTooLong
    );
    let mut cyclic_root = root;
    cyclic_root["schema_version"] = json!("1.1");
    cyclic_root["replaces_decision_id"] = json!("CG-FP-3");
    let cyclic_root_bytes = serde_json::to_vec(&cyclic_root).unwrap();
    let cyclic_root_snapshot = json!({
        "schema_version":"1.1", "policy_revision":"policy-r40",
        "max_lifetime_seconds":2000000_u64,
        "decisions":[{"id":"CG-FP-1","sha256":sha(&cyclic_root_bytes)}],
        "revoked_decision_ids":["CG-FP-3"]
    });
    let cyclic_root_snapshot_bytes = serde_json::to_vec(&cyclic_root_snapshot).unwrap();
    let cyclic_root_pin = sha(&cyclic_root_snapshot_bytes);
    assert_eq!(
        check(&[
            middle_input,
            PriorApprovalInput {
                decision_bytes: &cyclic_root_bytes,
                snapshot_bytes: &cyclic_root_snapshot_bytes,
                expected_snapshot_sha256: Some(&cyclic_root_pin),
            },
        ]),
        SnapshotResolution::RevisionChainCycle
    );
}

#[test]
fn malformed_revocation_sets_fail_closed() {
    let bytes = serde_json::to_vec(&candidate()).unwrap();
    let mut duplicate = snapshot(&bytes);
    duplicate["schema_version"] = json!("1.1");
    duplicate["revoked_decision_ids"] = json!(["CG-FP-1", "CG-FP-1"]);
    assert_eq!(
        bind(&bytes, &duplicate, None),
        SnapshotResolution::InvalidSnapshot
    );
    let mut broad = snapshot(&bytes);
    broad["schema_version"] = json!("1.1");
    broad["revoked_decision_ids"] = json!(["CG-FP-*"]);
    assert_eq!(
        bind(&bytes, &broad, None),
        SnapshotResolution::InvalidSnapshot
    );
    let mut legacy = snapshot(&bytes);
    legacy["revoked_decision_ids"] = json!(["CG-FP-1"]);
    assert_eq!(
        bind(&bytes, &legacy, None),
        SnapshotResolution::InvalidSnapshot
    );
    let mut missing = snapshot(&bytes);
    missing["schema_version"] = json!("1.1");
    assert_eq!(
        bind(&bytes, &missing, None),
        SnapshotResolution::InvalidSnapshot
    );
}

#[test]
fn published_snapshot_schema_is_strict() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/approval-snapshot.schema.json"
    ))
    .expect("快照 schema JSON");
    assert_eq!(schema["$id"], "urn:codeguard:schema:approval-snapshot:1.0");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["decisions"]["items"]["additionalProperties"],
        false
    );
    let revised: Value = serde_json::from_str(include_str!(
        "../../../schemas/approval-snapshot-v1.1.schema.json"
    ))
    .expect("修订快照 schema JSON");
    assert_eq!(revised["additionalProperties"], false);
    assert!(
        revised["required"]
            .as_array()
            .unwrap()
            .contains(&json!("revoked_decision_ids"))
    );
    assert_eq!(
        revised["properties"]["revoked_decision_ids"]["uniqueItems"],
        true
    );
}
