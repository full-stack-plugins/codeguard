use codeguard_core::{
    HookEvent, HookScopeResolutionReason, HookTriggerAction, HookTriggerInput, HookWriteOutcome,
    plan_hook_trigger,
};

fn request(event: HookEvent) -> HookTriggerInput {
    HookTriggerInput {
        event,
        changed_paths: Vec::new(),
        task_id: None,
        write_outcome: HookWriteOutcome::Unknown,
        host_claims_blocking: true,
    }
}

#[test]
fn startup_and_stop_do_not_execute_quality_checks() {
    let startup = plan_hook_trigger(&request(HookEvent::SessionStart)).unwrap();
    assert_eq!(startup.action, HookTriggerAction::DiscoverProject);
    assert!(!startup.requires_git_snapshot);
    let stop = plan_hook_trigger(&request(HookEvent::Stop)).unwrap();
    assert_eq!(stop.action, HookTriggerAction::ShowSummary);
    assert!(!stop.may_claim_delivery);
    let prompt = plan_hook_trigger(&request(HookEvent::PromptSubmitted)).unwrap();
    assert_eq!(prompt.action, HookTriggerAction::ShowIntentGuidance);
    assert!(!prompt.requires_git_snapshot);
    assert!(!prompt.may_claim_delivery);
}

#[test]
fn successful_edit_requests_only_file_feedback() {
    let mut input = request(HookEvent::FileChanged);
    input.write_outcome = HookWriteOutcome::Confirmed;
    input.changed_paths = vec!["src/A.java".into(), "src/B.java".into()];
    let plan = plan_hook_trigger(&input).unwrap();
    assert_eq!(plan.action, HookTriggerAction::FastFileCheck);
    assert_eq!(plan.target_paths, input.changed_paths);
    assert!(plan.soft_result_reuse_candidate);
    assert!(!plan.may_claim_delivery);
    assert!(!plan.requires_git_snapshot);
}

#[test]
fn bulk_edit_and_long_paths_request_scope_resolution_without_truncating_checks() {
    let mut input = request(HookEvent::FileChanged);
    input.write_outcome = HookWriteOutcome::Confirmed;
    input.changed_paths = (0..9)
        .map(|index| format!("src/File{index}.java"))
        .collect();
    let bulk = plan_hook_trigger(&input).unwrap();
    assert_eq!(bulk.action, HookTriggerAction::ResolveChangedScope);
    assert_eq!(
        bulk.scope_resolution_reason,
        Some(HookScopeResolutionReason::FastScopeBudgetExceeded)
    );
    assert!(bulk.target_paths.is_empty());
    assert!(!bulk.soft_result_reuse_candidate);
    assert!(!bulk.may_claim_delivery);

    input.changed_paths = vec![format!("src/{}.java", "a".repeat(513))];
    let long = plan_hook_trigger(&input).unwrap();
    assert_eq!(long.action, HookTriggerAction::ResolveChangedScope);
    assert_eq!(
        long.scope_resolution_reason,
        Some(HookScopeResolutionReason::FastScopeBudgetExceeded)
    );
    assert!(long.target_paths.is_empty());

    input.changed_paths = vec!["src/Same.java".into(); 9];
    let duplicate = plan_hook_trigger(&input).unwrap();
    assert_eq!(duplicate.action, HookTriggerAction::FastFileCheck);
    assert_eq!(duplicate.target_paths, ["src/Same.java"]);

    input.changed_paths = (0..5)
        .map(|index| format!("src/{index}{}.java", "a".repeat(495)))
        .collect();
    let total = plan_hook_trigger(&input).unwrap();
    assert_eq!(total.action, HookTriggerAction::ResolveChangedScope);
    assert_eq!(
        total.scope_resolution_reason,
        Some(HookScopeResolutionReason::FastScopeBudgetExceeded)
    );
    assert!(total.target_paths.is_empty());

    input.changed_paths = (0..9)
        .map(|index| format!("src/File{index}.java"))
        .collect();
    input.changed_paths.push("../outside.java".into());
    assert!(plan_hook_trigger(&input).is_err());

    input.write_outcome = HookWriteOutcome::Failed;
    input.changed_paths.pop();
    assert_eq!(
        plan_hook_trigger(&input).unwrap().action,
        HookTriggerAction::NoCheck
    );
}

#[test]
fn uncertain_edit_or_missing_target_cannot_be_silently_skipped() {
    let mut input = request(HookEvent::FileChanged);
    input.write_outcome = HookWriteOutcome::Confirmed;
    assert_eq!(
        plan_hook_trigger(&input).unwrap().action,
        HookTriggerAction::ResolveChangedScope
    );
    assert_eq!(
        plan_hook_trigger(&input).unwrap().scope_resolution_reason,
        Some(HookScopeResolutionReason::ChangedPathsMissing)
    );
    input.changed_paths = vec!["src/A.java".into()];
    input.write_outcome = HookWriteOutcome::Unknown;
    assert_eq!(
        plan_hook_trigger(&input).unwrap().action,
        HookTriggerAction::ResolveChangedScope
    );
    assert_eq!(
        plan_hook_trigger(&input).unwrap().scope_resolution_reason,
        Some(HookScopeResolutionReason::WriteOutcomeUnknown)
    );
    input.changed_paths = vec!["../outside.java".into()];
    assert!(plan_hook_trigger(&input).is_err());
}

#[test]
fn failed_write_does_not_start_a_source_check() {
    let mut input = request(HookEvent::FileChanged);
    input.write_outcome = HookWriteOutcome::Failed;
    input.changed_paths = vec!["src/A.java".into()];
    let plan = plan_hook_trigger(&input).unwrap();
    assert_eq!(plan.action, HookTriggerAction::NoCheck);
    assert!(!plan.soft_result_reuse_candidate);
}

#[test]
fn repair_requests_original_task_verification() {
    let mut input = request(HookEvent::RepairReady);
    input.task_id = Some("CG-abc123".into());
    let plan = plan_hook_trigger(&input).unwrap();
    assert_eq!(plan.action, HookTriggerAction::VerifyTask);
    assert_eq!(plan.task_id.as_deref(), Some("CG-abc123"));
    assert!(!plan.soft_result_reuse_candidate);
    assert!(plan_hook_trigger(&request(HookEvent::RepairReady)).is_err());
}

#[test]
fn delivery_events_always_request_current_git_or_project_scope() {
    for (event, action) in [
        (HookEvent::PreCommit, HookTriggerAction::CommitGate),
        (HookEvent::PrePush, HookTriggerAction::PushGate),
        (HookEvent::Ci, HookTriggerAction::FullProjectCheck),
    ] {
        let mut input = request(event);
        input.changed_paths = vec!["src/A.java".into()];
        let plan = plan_hook_trigger(&input).unwrap();
        assert_eq!(plan.action, action);
        assert!(
            plan.target_paths.is_empty(),
            "caller paths cannot define delivery scope"
        );
        assert!(!plan.soft_result_reuse_candidate);
        assert_eq!(plan.requires_git_snapshot, event != HookEvent::Ci);
        assert!(
            !plan.may_claim_delivery,
            "planning is never proof of a gate"
        );
    }
}

#[test]
fn nonblocking_host_cannot_claim_a_strict_gate() {
    let mut input = request(HookEvent::PreCommit);
    input.host_claims_blocking = false;
    let plan = plan_hook_trigger(&input).unwrap();
    assert_eq!(plan.action, HookTriggerAction::CommitGate);
    assert!(!plan.host_blocking_claimed);
    assert!(!plan.may_claim_delivery);
}
