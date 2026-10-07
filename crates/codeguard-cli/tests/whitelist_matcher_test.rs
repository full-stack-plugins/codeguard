//! 白名单匹配器测试

use codeguard_cli::whitelist_matcher::*;

fn create_entry() -> WhitelistEntry {
    WhitelistEntry {
        rule_id: "rule-1".into(),
        target: "file.rs".into(),
        tool_identity: "tool-1".into(),
        approval_identity: "approval-1".into(),
    }
}

#[test]
fn match_entry_success() {
    let entry = create_entry();
    let result = WhitelistMatcher::match_entry(&entry, "rule-1", "file.rs", "tool-1", "approval-1");
    
    assert!(result.matched);
    assert!(result.reason.is_none());
}

#[test]
fn match_entry_mismatch() {
    let entry = create_entry();
    let result = WhitelistMatcher::match_entry(&entry, "rule-2", "file.rs", "tool-1", "approval-1");
    
    assert!(!result.matched);
    assert_eq!(result.reason, Some("identity_mismatch".into()));
}

#[test]
fn validate_no_wildcard() {
    let entry = create_entry();
    assert!(WhitelistMatcher::validate_no_wildcard(&entry));
}

#[test]
fn validate_wildcard_rejected() {
    let entry = WhitelistEntry {
        rule_id: "*".into(),
        target: "file.rs".into(),
        tool_identity: "tool-1".into(),
        approval_identity: "approval-1".into(),
    };
    assert!(!WhitelistMatcher::validate_no_wildcard(&entry));
}

#[test]
fn validate_finding_retained() {
    assert!(WhitelistMatcher::validate_finding_retained());
}
