use codeguard_cli::false_positive_decision::{
    CandidateContext, CandidateResolution, classify_false_positive_candidates,
    parse_false_positive_decision_candidate,
};
use serde_json::{Value, json};

fn valid() -> Value {
    json!({
        "schema_version":"1.0",
        "kind":"false_positive",
        "id":"CG-FP-1",
        "identity":{
            "finding_id":"CG-F401-1",
            "checker_id":"ruff",
            "native_rule_id":"F401",
            "category":"lint",
            "target":{"kind":"source","path":"src/app.py","file_sha256":"a".repeat(64)},
            "finding_fingerprint":"b".repeat(64),
            "tool_sha256":"c".repeat(64),
            "adapter_sha256":"d".repeat(64),
            "rulepack_sha256":"e".repeat(64)
        },
        "reason_code":"native_false_positive",
        "rationale":"Ruff 在此生成代码中报告的 F401 与源码生成约定冲突",
        "reproducer_ref":"evidence:case-1",
        "approved_policy_revision":"policy-r42",
        "approval_ref":"review:123",
        "reviewer":"reviewer-1",
        "created_at":1790000000_u64,
        "expires_at":1791000000_u64
    })
}

fn parse(
    value: &Value,
) -> Result<codeguard_cli::false_positive_decision::FalsePositiveDecisionCandidate, String> {
    parse_false_positive_decision_candidate(&serde_json::to_vec(value).expect("测试 JSON"))
}

#[test]
fn valid_document_is_only_a_candidate_even_with_approval_text() {
    let candidate = parse(&valid()).expect("结构完整的候选");
    assert_eq!(candidate.id, "CG-FP-1");
    assert_eq!(candidate.approved_policy_revision, "policy-r42");
    assert_eq!(candidate.expires_at, 1791000000);
}

#[test]
fn missing_or_unbounded_expiry_is_rejected() {
    let mut document = valid();
    document.as_object_mut().unwrap().remove("expires_at");
    assert!(parse(&document).is_err());
    document = valid();
    document["expires_at"] = document["created_at"].clone();
    assert!(parse(&document).is_err());
    document["expires_at"] = json!(0);
    assert!(parse(&document).is_err());
}

#[test]
fn approval_flag_cannot_be_smuggled_into_the_document() {
    let mut document = valid();
    document["approved"] = json!(true);
    assert!(parse(&document).is_err());
    document = valid();
    document["identity"]["target"]["ignore_all"] = json!(true);
    assert!(parse(&document).is_err());
}

#[test]
fn broad_or_changed_identity_cannot_be_a_candidate() {
    let mut document = valid();
    document["identity"]["target"]["path"] = json!("src/**/*.py");
    assert!(parse(&document).is_err());
    document = valid();
    document["identity"]["tool_sha256"] = json!("bad-digest");
    assert!(parse(&document).is_err());
    document = valid();
    document["identity"]["target"]["path"] = json!("../src/app.py");
    assert!(parse(&document).is_err());
}

#[test]
fn wrong_kind_version_or_missing_review_basis_is_rejected() {
    let mut document = valid();
    document["kind"] = json!("accepted_risk");
    assert!(parse(&document).is_err());
    document = valid();
    document["schema_version"] = json!("2.0");
    assert!(parse(&document).is_err());
    document = valid();
    document["reproducer_ref"] = json!("");
    assert!(parse(&document).is_err());
    document = valid();
    document["approval_ref"] = json!("");
    assert!(parse(&document).is_err());
}

#[test]
fn replacement_candidate_requires_a_distinct_prior_decision_reference() {
    let mut document = valid();
    document["schema_version"] = json!("1.1");
    assert!(parse(&document).is_err());
    document["replaces_decision_id"] = json!("CG-FP-1");
    assert!(parse(&document).is_err());
    document["id"] = json!("CG-FP-2");
    assert_eq!(
        parse(&document).unwrap().replaces_decision_id.as_deref(),
        Some("CG-FP-1")
    );
    document["replaces_decision_id"] = json!("CG-FP-*");
    assert!(parse(&document).is_err());
    document["schema_version"] = json!("1.0");
    assert!(parse(&document).is_err());
}

#[test]
fn published_schema_has_strict_nested_shape() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/false-positive-decision.schema.json"
    ))
    .expect("白名单 schema JSON");
    assert_eq!(
        schema["$id"],
        "urn:codeguard:schema:false-positive-decision:1.0"
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["$defs"]["identity"]["additionalProperties"], false);
    assert_eq!(
        schema["$defs"]["source_target"]["additionalProperties"],
        false
    );
    let replacement: Value = serde_json::from_str(include_str!(
        "../../../schemas/false-positive-decision-v1.1.schema.json"
    ))
    .expect("替代候选 schema JSON");
    assert_eq!(
        replacement["$id"],
        "urn:codeguard:schema:false-positive-decision:1.1"
    );
    assert_eq!(replacement["additionalProperties"], false);
    assert!(
        replacement["required"]
            .as_array()
            .unwrap()
            .contains(&json!("replaces_decision_id"))
    );
}

#[test]
fn exact_candidate_needs_separate_authority_verification() {
    let candidate = parse(&valid()).expect("候选");
    let context = CandidateContext {
        policy_revision: Some("policy-r42"),
        now_unix: Some(1790500000),
        max_lifetime_seconds: Some(2000000),
    };
    assert_eq!(
        classify_false_positive_candidates(
            &candidate.identity,
            std::slice::from_ref(&candidate),
            context
        ),
        CandidateResolution::ReadyForAuthorityCheck { index: 0 }
    );
    let mut other = candidate.identity.clone();
    other.native_rule_id = "F402".into();
    assert_eq!(
        classify_false_positive_candidates(&other, &[candidate], context),
        CandidateResolution::NoMatch
    );
}

#[test]
fn duplicate_exact_candidates_fail_closed_before_expiry_selection() {
    let first = parse(&valid()).expect("首条候选");
    let mut second = first.clone();
    second.id = "CG-FP-2".into();
    second.expires_at = first.created_at + 1;
    let context = CandidateContext {
        policy_revision: Some("policy-r42"),
        now_unix: Some(1790500000),
        max_lifetime_seconds: Some(2000000),
    };
    assert_eq!(
        classify_false_positive_candidates(&first.identity, &[first.clone(), second], context),
        CandidateResolution::ConflictingCandidates
    );
}

#[test]
fn same_finding_id_with_different_target_cannot_reach_authority_check() {
    let first = parse(&valid()).expect("首条候选");
    let mut second = first.clone();
    second.id = "CG-FP-2".into();
    if let codeguard_core::AllowlistTarget::Source { path, .. } = &mut second.identity.target {
        *path = "src/other.py".into();
    }
    let context = CandidateContext {
        policy_revision: Some("policy-r42"),
        now_unix: Some(1790500000),
        max_lifetime_seconds: Some(2000000),
    };
    assert_eq!(
        classify_false_positive_candidates(&first.identity, &[first.clone(), second], context),
        CandidateResolution::ConflictingCandidates
    );
}

#[test]
fn unknown_clock_or_policy_limit_cannot_validate_candidate() {
    let candidate = parse(&valid()).expect("候选");
    for context in [
        CandidateContext {
            policy_revision: Some("policy-r42"),
            now_unix: None,
            max_lifetime_seconds: Some(2000000),
        },
        CandidateContext {
            policy_revision: Some("policy-r42"),
            now_unix: Some(1790500000),
            max_lifetime_seconds: None,
        },
        CandidateContext {
            policy_revision: None,
            now_unix: Some(1790500000),
            max_lifetime_seconds: Some(2000000),
        },
    ] {
        assert_eq!(
            classify_false_positive_candidates(
                &candidate.identity,
                std::slice::from_ref(&candidate),
                context
            ),
            CandidateResolution::UnverifiableContext
        );
    }
}

#[test]
fn expiry_future_start_max_lifetime_and_revision_mismatch_are_distinct() {
    let candidate = parse(&valid()).expect("候选");
    let classify = |policy_revision, now_unix, max_lifetime_seconds| {
        classify_false_positive_candidates(
            &candidate.identity,
            std::slice::from_ref(&candidate),
            CandidateContext {
                policy_revision: Some(policy_revision),
                now_unix: Some(now_unix),
                max_lifetime_seconds: Some(max_lifetime_seconds),
            },
        )
    };
    assert_eq!(
        classify("policy-other", 1790500000, 2000000),
        CandidateResolution::PolicyRevisionMismatch
    );
    assert_eq!(
        classify("policy-r42", 1789999999, 2000000),
        CandidateResolution::NotYetValid
    );
    assert_eq!(
        classify("policy-r42", 1791000000, 2000000),
        CandidateResolution::Expired
    );
    assert_eq!(
        classify("policy-r42", 1790500000, 100),
        CandidateResolution::LifetimeExceeded
    );
}
