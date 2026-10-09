//! 点前缀策略测试

use codeguard_cli::dot_prefix_policy::*;

fn create_entry(pattern: &str, exclude: bool, authorized: bool) -> PolicyEntry {
    PolicyEntry {
        pattern: pattern.into(),
        exclude,
        authorized,
    }
}

#[test]
fn validate_authorized_exclude() {
    let entries = vec![create_entry(".env", true, true)];
    let result = DotPrefixPolicy::validate(&entries);
    assert!(result.valid);
    assert_eq!(result.unauthorized_excludes, 0);
}

#[test]
fn validate_unauthorized_exclude() {
    let entries = vec![create_entry(".env", true, false)];
    let result = DotPrefixPolicy::validate(&entries);
    assert!(!result.valid);
    assert_eq!(result.unauthorized_excludes, 1);
}

#[test]
fn check_f18() {
    let entries = vec![create_entry(".env", true, true)];
    assert!(DotPrefixPolicy::check_f18(&entries));
}

#[test]
fn check_f18_unauthorized() {
    let entries = vec![create_entry(".env", true, false)];
    assert!(!DotPrefixPolicy::check_f18(&entries));
}
