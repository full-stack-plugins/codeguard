//! 隐私验证测试

use codeguard_cli::privacy_verification::*;

#[test]
fn logs_not_in_git() {
    assert!(PrivacyVerifier::validate_logs_not_in_git());
}

#[test]
fn deletion_not_affect_gate() {
    assert!(PrivacyVerifier::validate_deletion_not_affect_gate());
}

#[test]
fn cross_machine_recheck() {
    assert!(PrivacyVerifier::validate_cross_machine_recheck());
}

#[test]
fn verify_all_privacy() {
    let result = PrivacyVerifier::verify();
    assert!(result.logs_not_in_git);
    assert!(!result.deletion_affects_gate);
    assert!(result.cross_machine_recheckable);
}
