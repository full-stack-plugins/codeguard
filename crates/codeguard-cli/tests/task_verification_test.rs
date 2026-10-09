//! Task verification 测试

use codeguard_cli::task_verification::*;

#[test]
fn verify_resolved_with_identity() {
    let record = TaskVerification::verify("task-1", VerificationResult::Resolved, "abc123");
    assert!(record.auto_closed);
    assert!(TaskVerification::can_close(&record));
}

#[test]
fn verify_resolved_without_identity() {
    let record = TaskVerification::verify("task-1", VerificationResult::Resolved, "");
    assert!(!record.auto_closed);
    assert!(!TaskVerification::can_close(&record));
}

#[test]
fn verify_still_present() {
    let record = TaskVerification::verify("task-1", VerificationResult::StillPresent, "abc123");
    assert!(!record.auto_closed);
    assert!(TaskVerification::can_reopen(&record));
}

#[test]
fn verify_recurrence() {
    let record = TaskVerification::verify("task-1", VerificationResult::Recurrence, "abc123");
    assert!(TaskVerification::can_reopen(&record));
}

#[test]
fn verify_incomplete() {
    let record = TaskVerification::verify("task-1", VerificationResult::Incomplete, "abc123");
    assert!(!record.auto_closed);
    assert!(!TaskVerification::can_close(&record));
}
