//! 处置归因测试

use codeguard_cli::disposition_attribution::*;

#[test]
fn code_fix_counts() {
    let result = DispositionAttributor::attribute(Disposition::Code, None);
    assert!(result.counts_as_code_fix);
}

#[test]
fn policy_resolved_not_code_fix() {
    let result = DispositionAttributor::attribute(Disposition::Policy, None);
    assert!(!result.counts_as_code_fix);
}

#[test]
fn exception_retains_unresolved() {
    let exception = ExceptionLabel {
        id: "exc-1".into(),
        expires_at: 100,
        retains_unresolved: true,
    };
    
    let result = DispositionAttributor::attribute(Disposition::Code, Some(exception));
    assert!(result.exception.is_some());
}

#[test]
fn validate_exception() {
    let exception = ExceptionLabel {
        id: "exc-1".into(),
        expires_at: 100,
        retains_unresolved: true,
    };
    
    assert!(DispositionAttributor::validate_exception(&exception));
}

#[test]
fn validate_exception_no_expiry() {
    let exception = ExceptionLabel {
        id: "exc-1".into(),
        expires_at: 0,
        retains_unresolved: true,
    };
    
    assert!(!DispositionAttributor::validate_exception(&exception));
}

#[test]
fn dependency_disposition() {
    let result = DispositionAttributor::attribute(Disposition::Dependency, None);
    assert!(result.counts_as_code_fix);
}
