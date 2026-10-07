//! 自有产物范围测试

use codeguard_cli::artifact_scope::*;

fn create_rule(pattern: &str, exclude: bool, recursive: bool) -> ScopeRule {
    ScopeRule {
        pattern: pattern.into(),
        exclude,
        recursive,
    }
}

#[test]
fn user_source_in_scope() {
    let rules = vec![];
    let result = ScopeChecker::check_scope("codeguard/src/main.rs", &rules, true, false);
    
    assert!(result.in_scope);
    assert_eq!(result.reason, Some("user_source".into()));
}

#[test]
fn secret_blocked() {
    let rules = vec![];
    let result = ScopeChecker::check_scope(".env", &rules, false, true);
    
    assert!(!result.in_scope);
    assert_eq!(result.reason, Some("secret_detected".into()));
}

#[test]
fn excluded_file_out_of_scope() {
    let rules = vec![create_rule("vendor/", true, false)];
    let result = ScopeChecker::check_scope("vendor/lib.rs", &rules, false, false);
    
    assert!(!result.in_scope);
}

#[test]
fn normal_file_in_scope() {
    let rules = vec![];
    let result = ScopeChecker::check_scope("src/main.rs", &rules, false, false);
    
    assert!(result.in_scope);
}

#[test]
fn no_recursive_scan_for_copies() {
    let rules = vec![create_rule("copy", false, false)];
    assert!(ScopeChecker::check_no_recursive_scan("copy/", &rules));
}

#[test]
fn validate_workspace_secrets() {
    let files = vec![
        ScopeCheckResult {
            file: ".env".into(),
            in_scope: false,
            reason: Some("secret_detected".into()),
        },
    ];
    
    assert!(ScopeChecker::validate_workspace(&files));
}
