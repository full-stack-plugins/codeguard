use codeguard_core::{
    HookEvent, HookTriggerAction, HookTriggerInput, HookWriteOutcome, plan_hook_trigger,
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
fn uncertain_edit_or_missing_target_cannot_be_silently_skipped() {
    let mut input = request(HookEvent::FileChanged);
    input.write_outcome = HookWriteOutcome::Confirmed;
    assert_eq!(
        plan_hook_trigger(&input).unwrap().action,
        HookTriggerAction::ResolveChangedScope
    );
    input.changed_paths = vec!["src/A.java".into()];
    input.write_outcome = HookWriteOutcome::Unknown;
    assert_eq!(
        plan_hook_trigger(&input).unwrap().action,
        HookTriggerAction::ResolveChangedScope
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
