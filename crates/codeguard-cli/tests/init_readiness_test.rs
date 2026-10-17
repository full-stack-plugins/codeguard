//! Init readiness 测试

use codeguard_cli::init_readiness::*;

#[test]
fn check_all_ready() {
    let required = vec![("jdk".to_string(), true)];
    let optional = vec![("node".to_string(), false)];
    
    let report = ReadinessChecker::check(&required, &optional);
    assert_eq!(report.status, ReadinessStatus::Ready);
}

#[test]
fn check_incomplete() {
    let required = vec![("jdk".to_string(), false)];
    let optional = vec![];
    
    let report = ReadinessChecker::check(&required, &optional);
    assert_eq!(report.status, ReadinessStatus::Incomplete);
}

#[test]
fn optional_not_blocking() {
    let optional = vec![("node".to_string(), false)];
    assert!(ReadinessChecker::validate_optional_not_blocking(&optional));
}

#[test]
fn init_not_gate() {
    let report = ReadinessReport {
        status: ReadinessStatus::Incomplete,
        next_actions: vec![],
        init_success: true,
    };
    
    assert!(ReadinessChecker::validate_init_not_gate(&report));
}
