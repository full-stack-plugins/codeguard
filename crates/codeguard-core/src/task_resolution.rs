use crate::{
    ResolutionCause, ResolutionEvidence, ResolutionOutcome, TaskIdentity, TaskLifecycleEvent,
    TaskLifecycleKind, TaskLifecycleState, TaskLifecycleView,
};
use std::collections::{BTreeMap, BTreeSet};

/// 核对限定任务的原生关闭证据；参数是独立固定任务和已由应用边界核验的观察。
/// 返回解决、仍存在、误报调查或未完成；不能签发项目 allow。
#[must_use]
pub fn evaluate_resolution(
    expected: &TaskIdentity,
    evidence: &ResolutionEvidence,
) -> ResolutionOutcome {
    if expected != &evidence.identity || !valid_identity(expected) {
        return ResolutionOutcome::Incomplete("task_resolution_identity_mismatch");
    }
    if !evidence.policy_verified || !token(&evidence.policy_revision) {
        return ResolutionOutcome::Incomplete("task_resolution_policy_unverified");
    }
    for value in [
        &evidence.original_source_sha256,
        &evidence.current_source_sha256,
        &evidence.native_report_sha256,
        &evidence.tool_sha256,
        &evidence.adapter_sha256,
        &evidence.rulepack_sha256,
        &evidence.policy_sha256,
    ] {
        if !digest(value) {
            return ResolutionOutcome::Incomplete("task_resolution_digest_invalid");
        }
    }
    if !evidence.inputs_current {
        return ResolutionOutcome::Incomplete("task_resolution_inputs_stale");
    }
    if evidence.target_removed
        || matches!(
            evidence.cause,
            ResolutionCause::TargetRemoved | ResolutionCause::PolicyResolved
        )
    {
        return ResolutionOutcome::Incomplete("task_resolution_disposition_evidence_required");
    }
    if evidence.suppression_changed {
        return ResolutionOutcome::Incomplete("task_resolution_suppression_changed");
    }
    if !evidence.native_completed || !evidence.target_covered || !evidence.original_rule_checked {
        return ResolutionOutcome::Incomplete("task_resolution_native_coverage_incomplete");
    }
    if evidence.issue_still_present {
        return ResolutionOutcome::StillOpen;
    }
    if matches!(
        evidence.cause,
        ResolutionCause::CodeFixed | ResolutionCause::DependencyFixed
    ) && evidence.original_source_sha256 == evidence.current_source_sha256
    {
        return ResolutionOutcome::FalsePositiveReviewRequired;
    }
    ResolutionOutcome::Resolved(evidence.cause)
}

/// 按父关系重放一项任务的有界历史，不依据时间戳选最后一份关闭记录。
/// `verified_resolution_event_ids` 必须来自独立复核的精确事件及证据，不能读取项目自报授权。
/// 返回唯一链状态或核对要求；原生 recurrence 可以重开，未核验关闭保持待验证。
#[must_use]
pub fn reduce_task_lifecycle(
    expected: &TaskIdentity,
    events: &[TaskLifecycleEvent],
    verified_resolution_event_ids: &[String],
) -> TaskLifecycleView {
    let fail = |reason| TaskLifecycleView {
        state: TaskLifecycleState::ReconciliationRequired,
        tip_event_id: None,
        reason,
    };
    if !valid_identity(expected)
        || events.len() > 1000
        || verified_resolution_event_ids.len() > 1000
    {
        return fail("task_lifecycle_budget_or_identity_invalid");
    }
    if events.is_empty() {
        return TaskLifecycleView {
            state: TaskLifecycleState::VerificationRequired,
            tip_event_id: None,
            reason: "task_lifecycle_history_missing",
        };
    }
    let mut by_id = BTreeMap::new();
    let mut children = BTreeMap::new();
    let mut root = None;
    for event in events {
        if event.identity != *expected
            || !token(&event.event_id)
            || by_id.insert(event.event_id.as_str(), event).is_some()
        {
            return fail("task_lifecycle_event_identity_conflict");
        }
        match &event.parent_event_id {
            Some(parent) if token(parent) => {
                if children
                    .insert(parent.as_str(), event.event_id.as_str())
                    .is_some()
                {
                    return fail("task_lifecycle_fork");
                }
            }
            Some(_) => return fail("task_lifecycle_parent_invalid"),
            None => {
                if root.replace(event.event_id.as_str()).is_some() {
                    return fail("task_lifecycle_multiple_roots");
                }
            }
        }
    }
    let verified: BTreeSet<_> = verified_resolution_event_ids
        .iter()
        .map(String::as_str)
        .collect();
    if verified.len() != verified_resolution_event_ids.len()
        || verified.iter().any(|id| {
            !by_id
                .get(id)
                .is_some_and(|e| matches!(e.kind, TaskLifecycleKind::Resolved { .. }))
        })
    {
        return fail("task_lifecycle_verification_binding_invalid");
    }
    if children.keys().any(|id| !by_id.contains_key(id)) {
        return fail("task_lifecycle_parent_missing");
    }
    let Some(mut cursor) = root else {
        return fail("task_lifecycle_cycle_or_root_missing");
    };
    if !matches!(by_id[cursor].kind, TaskLifecycleKind::Observed) {
        return fail("task_lifecycle_root_not_observed");
    }
    let mut seen = BTreeSet::new();
    let mut state;
    loop {
        if !seen.insert(cursor) {
            return fail("task_lifecycle_cycle");
        }
        match &by_id[cursor].kind {
            TaskLifecycleKind::Observed => {
                if by_id[cursor]
                    .parent_event_id
                    .as_deref()
                    .and_then(|parent| by_id.get(parent))
                    .is_some_and(|p| matches!(p.kind, TaskLifecycleKind::Resolved { .. }))
                {
                    return fail("task_lifecycle_reopen_event_required");
                }
                state = TaskLifecycleState::Open;
            }
            TaskLifecycleKind::Resolved {
                evidence_sha256, ..
            } => {
                if !digest(evidence_sha256)
                    || by_id[cursor]
                        .parent_event_id
                        .as_deref()
                        .and_then(|parent| by_id.get(parent))
                        .is_some_and(|p| matches!(p.kind, TaskLifecycleKind::Resolved { .. }))
                {
                    return fail("task_lifecycle_resolution_transition_invalid");
                }
                state = if verified.contains(cursor) {
                    TaskLifecycleState::Resolved
                } else {
                    TaskLifecycleState::VerificationRequired
                };
            }
            TaskLifecycleKind::VerificationRequired {
                evidence_sha256,
                reason_code,
            } => {
                if !digest(evidence_sha256) || !token(reason_code) {
                    return fail("task_lifecycle_verification_event_invalid");
                }
                state = TaskLifecycleState::VerificationRequired;
            }
            TaskLifecycleKind::Reopened => {
                let parent = by_id[cursor].parent_event_id.as_deref().unwrap_or("");
                if !by_id
                    .get(parent)
                    .is_some_and(|p| matches!(p.kind, TaskLifecycleKind::Resolved { .. }))
                {
                    return fail("task_lifecycle_reopen_parent_invalid");
                }
                state = TaskLifecycleState::Open;
            }
        }
        match children.get(cursor) {
            Some(child) => cursor = child,
            None => break,
        }
    }
    if seen.len() != events.len() {
        return fail("task_lifecycle_disconnected_events");
    }
    TaskLifecycleView {
        state,
        tip_event_id: Some(cursor.to_owned()),
        reason: if state == TaskLifecycleState::VerificationRequired {
            "task_lifecycle_resolution_unverified"
        } else {
            "task_lifecycle_chain_valid"
        },
    }
}
fn token(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_identity(v: &TaskIdentity) -> bool {
    token(&v.workspace_id)
        && token(&v.task_id)
        && token(&v.checker_id)
        && !v.scope.is_empty()
        && v.scope.len() <= 4096
        && !v.scope.contains('\\')
        && !v.scope.contains(':')
        && v.scope
            .split('/')
            .all(|p| !matches!(p, "" | "." | "..") && !p.chars().any(char::is_control))
}
