//! Whitelist 命令测试

use codeguard_cli::whitelist_commands::*;

fn create_candidate(rule: &str, target: &str, has_identity: bool) -> WhitelistCandidate {
    WhitelistCandidate {
        rule_id: rule.into(),
        target: target.into(),
        has_complete_identity: has_identity,
    }
}

#[test]
fn list_candidates() {
    let candidates = vec![create_candidate("rule-1", "file.rs", true)];
    let ids = WhitelistCommands::list(&candidates);
    assert_eq!(ids, vec!["rule-1".to_string()]);
}

#[test]
fn explain_candidate() {
    let candidate = create_candidate("rule-1", "file.rs", true);
    let explanation = WhitelistCommands::explain(&candidate);
    assert!(explanation.contains("rule-1"));
}

#[test]
fn propose_complete() {
    let findings = vec![
        ("rule-1".to_string(), "file.rs".to_string(), true),
    ];
    
    let result = WhitelistCommands::propose(&findings);
    assert!(result.complete);
    assert!(result.missing_identity.is_none());
}

#[test]
fn propose_incomplete() {
    let findings = vec![
        ("rule-1".to_string(), "file.rs".to_string(), false),
    ];
    
    let result = WhitelistCommands::propose(&findings);
    assert!(!result.complete);
    assert_eq!(result.missing_identity, Some("rule-1".to_string()));
}
