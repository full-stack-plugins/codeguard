//! Zig 适用性测试

use codeguard_cli::zig_applicability::*;

#[test]
fn check_zig_applicability() {
    let slots = ZigApplicability::check();
    assert_eq!(slots.len(), 6);
}

#[test]
fn validate_no_missing_tool_excuse() {
    let slots = ZigApplicability::check();
    assert!(ZigApplicability::validate_no_missing_tool_excuse(&slots));
}
