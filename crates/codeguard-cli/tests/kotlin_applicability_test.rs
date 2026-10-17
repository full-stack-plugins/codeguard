//! Kotlin 适用性测试

use codeguard_cli::kotlin_applicability::*;

#[test]
fn check_kotlin_applicability() {
    let slots = KotlinApplicability::check();
    assert_eq!(slots.len(), 6);
}

#[test]
fn validate_no_missing_tool_excuse() {
    let slots = KotlinApplicability::check();
    assert!(KotlinApplicability::validate_no_missing_tool_excuse(&slots));
}
