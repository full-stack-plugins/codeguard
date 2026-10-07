//! 剩余语言适用性测试

use codeguard_cli::remaining_language_applicability::*;

#[test]
fn check_nix_applicability() {
    let slots = RemainingLanguageApplicability::check_nix();
    assert_eq!(slots.len(), 6);
    assert!(RemainingLanguageApplicability::validate_no_missing_tool_excuse(&slots));
}

#[test]
fn check_cuda_applicability() {
    let slots = RemainingLanguageApplicability::check_cuda();
    assert_eq!(slots.len(), 6);
    assert!(RemainingLanguageApplicability::validate_no_missing_tool_excuse(&slots));
}

#[test]
fn check_liquid_applicability() {
    let slots = RemainingLanguageApplicability::check_liquid();
    assert_eq!(slots.len(), 6);
    assert!(RemainingLanguageApplicability::validate_no_missing_tool_excuse(&slots));
}
