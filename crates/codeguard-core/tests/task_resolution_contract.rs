use codeguard_core::{
    ResolutionCause, ResolutionEvidence, ResolutionOutcome, TaskIdentity, TaskLifecycleEvent,
    TaskLifecycleKind, TaskLifecycleState, evaluate_resolution, reduce_task_lifecycle,
};

fn identity() -> TaskIdentity {
    TaskIdentity {
        workspace_id: "ws-one".into(),
        task_id: "CG-one".into(),
        checker_id: "syntax.native_confirmation".into(),
        scope: "app.zig".into(),
    }
}
fn evidence() -> ResolutionEvidence {
    ResolutionEvidence {
        identity: identity(),
        original_source_sha256: "a".repeat(64),
        current_source_sha256: "b".repeat(64),
        native_report_sha256: "c".repeat(64),
        tool_sha256: "d".repeat(64),
        adapter_sha256: "e".repeat(64),
        rulepack_sha256: "f".repeat(64),
        policy_sha256: "1".repeat(64),
        policy_revision: "p1".into(),
        native_completed: true,
        original_rule_checked: true,
        target_covered: true,
        inputs_current: true,
        policy_verified: true,
        issue_still_present: false,
        suppression_changed: false,
        target_removed: false,
        cause: ResolutionCause::CodeFixed,
    }
}
#[test]
fn complete_current_rule_evidence_resolves_only_the_target() {
    assert_eq!(
        evaluate_resolution(&identity(), &evidence()),
        ResolutionOutcome::Resolved(ResolutionCause::CodeFixed)
    );
}
#[test]
fn missing_identity_policy_or_coverage_never_resolves() {
    for i in 0..10 {
        let mut e = evidence();
        match i {
            0 => e.policy_verified = false,
            1 => e.target_covered = false,
            2 => e.native_completed = false,
            3 => e.original_rule_checked = false,
            4 => e.inputs_current = false,
            5 => e.suppression_changed = true,
            6 => e.tool_sha256.clear(),
            7 => e.policy_sha256.clear(),
            8 => e.identity.scope = "other.zig".into(),
            _ => e.policy_revision.clear(),
        }
        assert!(
            matches!(
                evaluate_resolution(&identity(), &e),
                ResolutionOutcome::Incomplete(_)
            ),
            "case {i}"
        );
    }
    let mut e = evidence();
    e.issue_still_present = true;
    assert_eq!(
        evaluate_resolution(&identity(), &e),
        ResolutionOutcome::StillOpen
    );
}
#[test]
fn unchanged_source_needs_false_positive_review_and_deletion_needs_approval() {
    let mut e = evidence();
    e.current_source_sha256 = e.original_source_sha256.clone();
    assert_eq!(
        evaluate_resolution(&identity(), &e),
        ResolutionOutcome::FalsePositiveReviewRequired
    );
    e.target_removed = true;
    assert!(matches!(
        evaluate_resolution(&identity(), &e),
        ResolutionOutcome::Incomplete(_)
    ));
}
fn event(id: &str, parent: Option<&str>, kind: TaskLifecycleKind) -> TaskLifecycleEvent {
    TaskLifecycleEvent {
        event_id: id.into(),
        parent_event_id: parent.map(str::to_owned),
        identity: identity(),
        kind,
    }
}
#[test]
fn verified_close_and_recurrence_replay_without_order_dependence() {
    let observed = event("observed", None, TaskLifecycleKind::Observed);
    let closed = event(
        "closed",
        Some("observed"),
        TaskLifecycleKind::Resolved {
            cause: ResolutionCause::CodeFixed,
            evidence_sha256: "1".repeat(64),
        },
    );
    let reopened = event("reopened", Some("closed"), TaskLifecycleKind::Reopened);
    let mut events = vec![closed.clone(), observed.clone()];
    assert_eq!(
        reduce_task_lifecycle(&identity(), &events, &["closed".into()]).state,
        TaskLifecycleState::Resolved
    );
    assert_eq!(
        reduce_task_lifecycle(&identity(), &events, &[]).state,
        TaskLifecycleState::VerificationRequired
    );
    events.insert(0, reopened);
    assert_eq!(
        reduce_task_lifecycle(&identity(), &events, &["closed".into()]).state,
        TaskLifecycleState::Open
    );
}
#[test]
fn forks_missing_parents_cycles_and_wrong_scope_require_reconciliation() {
    let root = event("root", None, TaskLifecycleKind::Observed);
    let close = event(
        "close",
        Some("root"),
        TaskLifecycleKind::Resolved {
            cause: ResolutionCause::CodeFixed,
            evidence_sha256: "1".repeat(64),
        },
    );
    let fork = event(
        "fork",
        Some("root"),
        TaskLifecycleKind::Resolved {
            cause: ResolutionCause::CodeFixed,
            evidence_sha256: "2".repeat(64),
        },
    );
    for events in [
        vec![root.clone(), close.clone(), fork],
        vec![close.clone()],
        vec![
            event("a", Some("b"), TaskLifecycleKind::Reopened),
            event("b", Some("a"), TaskLifecycleKind::Reopened),
        ],
        vec![root.clone(), root.clone()],
    ] {
        assert_eq!(
            reduce_task_lifecycle(&identity(), &events, &["close".into()]).state,
            TaskLifecycleState::ReconciliationRequired
        );
    }
    let mut wrong = close;
    wrong.identity.scope = "other.zig".into();
    assert_eq!(
        reduce_task_lifecycle(&identity(), &[root, wrong], &[]).state,
        TaskLifecycleState::ReconciliationRequired
    );
}

#[test]
fn unverified_close_cannot_be_overwritten_by_plain_observation_or_another_close() {
    let root = event("root", None, TaskLifecycleKind::Observed);
    let close = event(
        "close",
        Some("root"),
        TaskLifecycleKind::Resolved {
            cause: ResolutionCause::CodeFixed,
            evidence_sha256: "1".repeat(64),
        },
    );
    for next in [
        event("next", Some("close"), TaskLifecycleKind::Observed),
        event(
            "next",
            Some("close"),
            TaskLifecycleKind::Resolved {
                cause: ResolutionCause::CodeFixed,
                evidence_sha256: "2".repeat(64),
            },
        ),
    ] {
        assert_eq!(
            reduce_task_lifecycle(&identity(), &[root.clone(), close.clone(), next], &[]).state,
            TaskLifecycleState::ReconciliationRequired
        );
    }
}
#[test]
fn failed_recheck_invalidates_a_previous_close_without_erasing_history() {
    let root = event("root", None, TaskLifecycleKind::Observed);
    let close = event(
        "close",
        Some("root"),
        TaskLifecycleKind::Resolved {
            cause: ResolutionCause::CodeFixed,
            evidence_sha256: "1".repeat(64),
        },
    );
    let failed = event(
        "failed",
        Some("close"),
        TaskLifecycleKind::VerificationRequired {
            evidence_sha256: "2".repeat(64),
            reason_code: "native_incomplete".into(),
        },
    );
    let view = reduce_task_lifecycle(&identity(), &[failed, root, close], &["close".into()]);
    assert_eq!(view.state, TaskLifecycleState::VerificationRequired);
    assert_eq!(view.tip_event_id.as_deref(), Some("failed"));
}
