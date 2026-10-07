//! 误报调查测试

use codeguard_cli::false_positive_investigation::*;

#[test]
fn create_investigation_task() {
    let task = FalsePositiveInvestigation::create(
        "task-1",
        "evidence-1",
        "repro-1",
        "false_positive",
        "scope-1",
    );
    
    assert_eq!(task.id, "task-1");
    assert!(task.has_recheck_history);
}

#[test]
fn validate_completeness() {
    let task = FalsePositiveInvestigation::create(
        "task-1",
        "evidence-1",
        "repro-1",
        "false_positive",
        "scope-1",
    );
    
    assert!(FalsePositiveInvestigation::validate_completeness(&task));
}

#[test]
fn validate_no_self_authorization() {
    let task = FalsePositiveInvestigation::create(
        "task-1",
        "evidence-1",
        "repro-1",
        "false_positive",
        "scope-1",
    );
    
    assert!(FalsePositiveInvestigation::validate_no_self_authorization(&task));
}

#[test]
fn validate_self_authorization_rejected() {
    let task = FalsePositiveInvestigation::create(
        "task-1",
        "evidence-1",
        "repro-1",
        "self_approved",
        "scope-1",
    );
    
    assert!(!FalsePositiveInvestigation::validate_no_self_authorization(&task));
}
