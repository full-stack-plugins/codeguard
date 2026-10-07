//! 内容身份测试

use codeguard_runtime::content_identity::*;

#[test]
fn check_consistent() {
    let result = ContentIdentityChecker::check("hash-1", "hash-1");
    assert!(result.consistent);
    assert!(!result.has_side_effect);
}

#[test]
fn check_inconsistent() {
    let result = ContentIdentityChecker::check("hash-1", "hash-2");
    assert!(!result.consistent);
    assert!(result.has_side_effect);
    assert_eq!(result.reason, Some("content_changed".into()));
}

#[test]
fn validate_concurrent_edit_incomplete() {
    let result = ContentIdentityChecker::check("hash-1", "hash-2");
    assert!(ContentIdentityChecker::validate_concurrent_edit_incomplete(&result));
}
