//! 离线执行边界测试

use codeguard_runtime::offline_boundary::*;

#[test]
fn offline_mode_denies_network() {
    let checker = OfflineBoundaryChecker::new(true);
    assert_eq!(checker.check_network_access("example.com"), NetworkAccess::Denied);
}

#[test]
fn online_mode_allows_network() {
    let checker = OfflineBoundaryChecker::new(false);
    assert_eq!(checker.check_network_access("example.com"), NetworkAccess::Allowed);
}

#[test]
fn whitelist_allows_target() {
    let checker = OfflineBoundaryChecker::new(true);
    checker.add_allowed_target("trusted.com");
    assert_eq!(checker.check_network_access("trusted.com"), NetworkAccess::Allowed);
}

#[test]
fn verify_offline_boundary() {
    let checker = OfflineBoundaryChecker::new(true);
    assert!(checker.verify().is_ok());
    assert!(checker.can_guarantee_offline());
}

#[test]
fn pre_start_check_fails_without_verification() {
    let checker = OfflineBoundaryChecker::new(true);
    assert!(checker.pre_start_check().is_err());
}

#[test]
fn pre_start_check_passes_with_verification() {
    let checker = OfflineBoundaryChecker::new(true);
    checker.verify().unwrap();
    assert!(checker.pre_start_check().is_ok());
}

#[test]
fn trusted_policy_boundary() {
    let boundary = TrustedPolicyBoundary::new("policy-v1", "issuer-1");
    assert!(boundary.can_enter_project_domain());
    assert!(boundary.verify_issuer());
}

#[test]
fn trusted_policy_empty_issuer() {
    let boundary = TrustedPolicyBoundary::new("policy-v1", "");
    assert!(!boundary.verify_issuer());
}

#[test]
fn offline_boundary_incomplete_when_unverifiable() {
    // 无法保证时启动前 incomplete
    let checker = OfflineBoundaryChecker::new(true);
    // 未验证时 pre_start_check 失败
    assert!(checker.pre_start_check().is_err());
}
