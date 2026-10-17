//! 多语言适用性测试

use codeguard_cli::language_applicability::*;

#[test]
fn check_swift_applicability() {
    let slots = LanguageApplicability::check_swift();
    assert_eq!(slots.len(), 6);
    assert!(LanguageApplicability::validate_no_missing_tool_excuse(&slots));
}

#[test]
fn check_c_applicability() {
    let slots = LanguageApplicability::check_c();
    assert_eq!(slots.len(), 6);
    assert!(LanguageApplicability::validate_no_missing_tool_excuse(&slots));
}

#[test]
fn check_cpp_applicability() {
    let slots = LanguageApplicability::check_cpp();
    assert_eq!(slots.len(), 6);
}

#[test]
fn check_objc_applicability() {
    let slots = LanguageApplicability::check_objc();
    assert_eq!(slots.len(), 6);
    assert!(LanguageApplicability::validate_no_missing_tool_excuse(&slots));
}
