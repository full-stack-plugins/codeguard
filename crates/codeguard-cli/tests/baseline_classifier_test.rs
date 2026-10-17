//! 基线分类器测试

use codeguard_cli::baseline_classifier::*;

#[test]
fn classify_new_finding() {
    let result = BaselineClassifier::classify("file.rs", false);
    assert_eq!(result.class, FindingClass::New);
    assert!(result.blocks);
}

#[test]
fn classify_existing_finding() {
    let result = BaselineClassifier::classify("file.rs", true);
    assert_eq!(result.class, FindingClass::Existing);
    assert!(result.blocks);
}

#[test]
fn baseline_failure_not_eliminate() {
    assert!(BaselineClassifier::validate_baseline_failure_not_eliminate());
}
