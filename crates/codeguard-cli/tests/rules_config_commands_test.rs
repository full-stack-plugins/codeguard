//! Rules/config 命令测试

use codeguard_cli::rules_config_commands::*;

#[test]
fn list_rules() {
    let rules = vec![
        RuleInfo { id: "rule-1".into(), valid: true, approval_source: Some("approved".into()) },
    ];
    
    let ids = RulesCommands::list(&rules);
    assert_eq!(ids, vec!["rule-1".to_string()]);
}

#[test]
fn validate_config() {
    let result = RulesCommands::validate("some config");
    assert!(result.valid);
}

#[test]
fn validate_empty_config() {
    let result = RulesCommands::validate("");
    assert!(!result.valid);
}

#[test]
fn explain_rule() {
    let rule = RuleInfo { id: "rule-1".into(), valid: true, approval_source: Some("approved".into()) };
    let explanation = RulesCommands::explain(&rule);
    assert!(explanation.contains("rule-1"));
}

#[test]
fn validate_no_script_execution() {
    assert!(RulesCommands::validate_no_script_execution());
}
