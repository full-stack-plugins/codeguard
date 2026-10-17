//! 画像刷新测试

use codeguard_cli::profile_refresh::*;

#[test]
fn refresh_no_change() {
    let result = ProfileRefresher::refresh(
        &["mod-a".to_string()],
        &["mod-a".to_string()],
        false,
        false,
    );
    
    assert!(!result.invalidated);
    assert!(result.idempotent);
}

#[test]
fn refresh_new_module() {
    let result = ProfileRefresher::refresh(
        &["mod-a".to_string()],
        &["mod-a".to_string(), "mod-b".to_string()],
        false,
        false,
    );
    
    assert!(result.invalidated);
}

#[test]
fn refresh_lock_changed() {
    let result = ProfileRefresher::refresh(
        &["mod-a".to_string()],
        &["mod-a".to_string()],
        true,
        false,
    );
    
    assert!(result.invalidated);
}

#[test]
fn refresh_rules_changed() {
    let result = ProfileRefresher::refresh(
        &["mod-a".to_string()],
        &["mod-a".to_string()],
        false,
        true,
    );
    
    assert!(result.invalidated);
}

#[test]
fn validate_idempotent() {
    let result = ProfileRefresher::refresh(&[], &[], false, false);
    assert!(ProfileRefresher::validate_idempotent(&result));
}
