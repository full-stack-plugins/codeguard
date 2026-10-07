//! 状态 disposition 测试

use codeguard_cli::status_disposition::*;

#[test]
fn check_no_tasks() {
    let report = StatusChecker::check(false, false);
    assert_eq!(report.disposition, Disposition::NoTasks);
    assert!(report.next_action.is_some());
}

#[test]
fn check_expired() {
    let report = StatusChecker::check(true, true);
    assert_eq!(report.disposition, Disposition::NeedsVerification);
}

#[test]
fn check_actionable() {
    let report = StatusChecker::check(true, false);
    assert_eq!(report.disposition, Disposition::Actionable);
}

#[test]
fn validate_no_tasks_not_pass() {
    let report = StatusChecker::check(false, false);
    assert!(StatusChecker::validate_no_tasks_not_pass(&report));
}

#[test]
fn validate_expired_no_allow() {
    let report = StatusChecker::check(true, true);
    assert!(StatusChecker::validate_expired_no_allow(&report));
}
