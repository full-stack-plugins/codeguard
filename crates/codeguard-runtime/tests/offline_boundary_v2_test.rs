//! 离线执行边界测试

use codeguard_runtime::offline_boundary_v2::*;

#[test]
fn offline_mode_denies_network() {
    let boundary = OfflineBoundaryV2::new(true);
    assert_eq!(boundary.check_network_access("example.com"), NetworkAccess::Denied);
}

#[test]
fn online_mode_allows_network() {
    let boundary = OfflineBoundaryV2::new(false);
    assert_eq!(boundary.check_network_access("example.com"), NetworkAccess::Allowed);
}

#[test]
fn whitelist_allows_target() {
    let boundary = OfflineBoundaryV2::new(true);
    boundary.add_allowed_target("trusted.com");
    assert_eq!(boundary.check_network_access("trusted.com"), NetworkAccess::Allowed);
}

#[test]
fn pre_start_check_fails_without_verification() {
    let boundary = OfflineBoundaryV2::new(true);
    assert!(boundary.pre_start_check().is_err());
}

#[test]
fn pre_start_check_passes_with_verification() {
    let mut boundary = OfflineBoundaryV2::new(true);
    boundary.verify().unwrap();
    assert!(boundary.pre_start_check().is_ok());
}
