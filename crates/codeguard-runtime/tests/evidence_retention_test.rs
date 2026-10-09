//! 证据保留测试

use codeguard_runtime::evidence_retention::*;

fn create_policy() -> RetentionPolicy {
    RetentionPolicy {
        max_age_days: 30,
        protect_user_source: true,
        protect_tracked_history: true,
    }
}

#[test]
fn managed_cleanup_active_retained() {
    let entries = vec![
        ("e1".to_string(), true, false),  // active
        ("e2".to_string(), false, false), // inactive
    ];
    
    let result = EvidenceRetention::managed_cleanup(&entries, &create_policy());
    assert_eq!(result.retained, 1);
    assert_eq!(result.cleaned, 1);
}

#[test]
fn managed_cleanup_referenced_retained() {
    let entries = vec![
        ("e1".to_string(), false, true),  // referenced
        ("e2".to_string(), false, false), // not referenced
    ];
    
    let result = EvidenceRetention::managed_cleanup(&entries, &create_policy());
    assert_eq!(result.retained, 1);
    assert_eq!(result.cleaned, 1);
}

#[test]
fn validate_no_active_cleanup() {
    let entries = vec![
        ("e1".to_string(), true, false),
    ];
    
    assert!(EvidenceRetention::validate_no_active_cleanup(&entries));
}

#[test]
fn policy_protects_user_source() {
    let policy = create_policy();
    assert!(policy.protect_user_source);
    assert!(policy.protect_tracked_history);
}
