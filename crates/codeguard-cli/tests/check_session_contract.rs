use codeguard_core::{
    AllowlistDisposition, AllowlistTarget, CheckSessionInput, Completion, DeliveryDecision,
    DeliveryInput, FalsePositiveIdentity, Finding, FindingLocation, GateImpact,
    NativeCheckerBinding, ObligationEvidence, ObligationResult, ObligationSpec, Verdict,
    conclude_check,
};

fn obligation(id: &str, target: &str) -> ObligationSpec {
    ObligationSpec {
        id: id.into(),
        expected_targets: vec![target.into()],
    }
}

fn evidence(id: &str, target: &str, findings: Vec<Finding>) -> ObligationEvidence {
    ObligationEvidence {
        result: ObligationResult {
            id: id.into(),
            completion: Completion::Complete,
            reason: None,
            findings,
        },
        observed_targets: vec![target.into()],
    }
}

fn violation(id: &str) -> Finding {
    Finding {
        id: "f-1".into(),
        native_rule_id: "F401".into(),
        tool_id: "ruff".into(),
        severity: "error".into(),
        gate_impact: GateImpact::Blocking,
        message: "unused import".into(),
        obligation_id: id.into(),
        native_identity: Some(disposition().observed_identity),
        locations: vec![FindingLocation::Source {
            path: "a.py".into(),
            line: Some(1),
            column: None,
        }],
    }
}

fn disposition() -> AllowlistDisposition {
    let identity = FalsePositiveIdentity {
        finding_id: "f-1".into(),
        checker_id: "python.ruff".into(),
        native_rule_id: "F401".into(),
        category: "lint".into(),
        target: AllowlistTarget::Source {
            path: "a.py".into(),
            file_sha256: "a".repeat(64),
        },
        finding_fingerprint: "b".repeat(64),
        tool_sha256: "c".repeat(64),
        adapter_sha256: "d".repeat(64),
        rulepack_sha256: "e".repeat(64),
    };
    AllowlistDisposition {
        approval_scope: Some(codeguard_core::ApprovalScope {
            workspace_id: "workspace-one".into(),
            baseline_commit: "a".repeat(40),
        }),
        observed_identity: identity.clone(),
        decision_identity: identity,
        decision_id: "CG-FP-1".into(),
        approval_ref: "review:one".into(),
        approved_policy_revision: "policy-r1".into(),
        independent_approval_verified: true,
        observed_at: 100,
        expires_at: 200,
    }
}

fn base() -> CheckSessionInput {
    CheckSessionInput {
        delivery: DeliveryInput {
            approval_scope: Some(codeguard_core::ApprovalScope {
                workspace_id: "workspace-one".into(),
                baseline_commit: "a".repeat(40),
            }),
            full_project: true,
            discovery_complete: true,
            trusted_bindings_verified: true,
            gate_time_unix: Some(100),
            policy_revision: Some("policy-r1".into()),
            frozen_obligation_ids: vec!["python/lint".into()],
            obligations: vec![obligation("python/lint", "a.py")],
            evidence: vec![evidence("python/lint", "a.py", Vec::new())],
            allowlist_dispositions: Vec::new(),
            native_checker_bindings: vec![NativeCheckerBinding {
                checker_id: "python.ruff".into(),
                tool_id: "ruff".into(),
                category: "lint".into(),
                obligation_id: "python/lint".into(),
            }],
        },
        internal_error: false,
        cancelled: false,
    }
}

#[test]
fn an_unexecuted_required_obligation_keeps_a_confirmed_violation_and_returns_three() {
    let mut input = base();
    input.delivery.evidence[0]
        .result
        .findings
        .push(violation("python/lint"));
    input
        .delivery
        .obligations
        .push(obligation("python/cve", "lockfile"));
    input
        .delivery
        .frozen_obligation_ids
        .push("python/cve".into());
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::Incomplete);
    assert_eq!(actual.summary.verdict.exit_code(), 3);
    assert_eq!(actual.summary.findings[0].id, "f-1");
    assert_eq!(actual.summary.incomplete_obligation_ids, ["python/cve"]);
    assert_eq!(actual.gate.decision, DeliveryDecision::Incomplete);
}

#[test]
fn omitted_plan_obligation_stays_incomplete_even_when_remaining_task_passes() {
    let mut input = base();
    input.delivery.frozen_obligation_ids.push("java/cve".into());
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::Incomplete);
    assert_eq!(actual.summary.verdict.exit_code(), 3);
    assert_eq!(actual.summary.incomplete_obligation_ids, ["java/cve"]);
    assert_eq!(actual.gate.decision, DeliveryDecision::Incomplete);
}

#[test]
fn category_request_cannot_sign_delivery_allow() {
    let mut input = base();
    input.delivery.full_project = false;
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::Passed);
    assert_eq!(actual.gate.decision, DeliveryDecision::NotEvaluated);
}

#[test]
fn zero_target_complete_evidence_is_incomplete() {
    let mut input = base();
    input.delivery.obligations[0].expected_targets.clear();
    input.delivery.evidence[0].observed_targets.clear();
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::Incomplete);
    assert_eq!(actual.summary.incomplete_obligation_ids, ["python/lint"]);
}

#[test]
fn truly_empty_discovered_project_is_not_applicable_without_allow() {
    let mut input = base();
    input.delivery.obligations.clear();
    input.delivery.evidence.clear();
    input.delivery.frozen_obligation_ids.clear();
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::NotApplicable);
    assert_eq!(actual.gate.decision, DeliveryDecision::NotApplicable);
}

#[test]
fn cancellation_precedes_other_gaps_and_preserves_partial_findings() {
    let mut input = base();
    input.cancelled = true;
    input.internal_error = true;
    input.delivery.evidence[0]
        .result
        .findings
        .push(violation("python/lint"));
    let actual = conclude_check(&input);
    assert_eq!(actual.summary.verdict, Verdict::Cancelled);
    assert_eq!(actual.summary.verdict.exit_code(), 130);
    assert_eq!(actual.summary.findings.len(), 1);
}

#[test]
fn approved_false_positive_has_distinct_request_verdict_and_keeps_raw_finding() {
    let mut input = base();
    input.delivery.evidence[0]
        .result
        .findings
        .push(violation("python/lint"));
    input.delivery.allowlist_dispositions.push(disposition());
    let actual = conclude_check(&input);
    assert_eq!(actual.gate.decision, DeliveryDecision::AllowWithExceptions);
    assert_eq!(actual.summary.verdict, Verdict::PassedWithExceptions);
    assert_eq!(actual.summary.verdict.exit_code(), 0);
    assert_eq!(actual.summary.findings[0].id, "f-1");
}

#[test]
fn cancelled_or_failed_request_never_leaves_an_allow_gate() {
    let mut input = base();
    input.cancelled = true;
    let cancelled = conclude_check(&input);
    assert_eq!(cancelled.summary.verdict, Verdict::Cancelled);
    assert_eq!(cancelled.gate.decision, DeliveryDecision::Incomplete);
    input.cancelled = false;
    input.internal_error = true;
    let failed = conclude_check(&input);
    assert_eq!(failed.summary.verdict, Verdict::InternalError);
    assert_eq!(failed.gate.decision, DeliveryDecision::Incomplete);
}
