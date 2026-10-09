//! 基线分类测试

use codeguard_cli::baseline_classification::*;

fn create_baseline() -> Baseline {
    Baseline {
        entries: vec![
            BaselineEntry {
                file: "src/main.rs".into(),
                rule: "unused_variable".into(),
                line: 10,
                fingerprint: "src/main.rs:unused_variable:10".into(),
            },
        ],
        version: "1.0.0".into(),
    }
}

#[test]
fn classify_new_and_existing() {
    let baseline = create_baseline();
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
        ("src/main.rs".to_string(), "unused_variable".to_string(), 20),
    ];
    
    let result = BaselineClassifier::classify(&findings, &baseline);
    
    assert_eq!(result.new_count, 1);
    assert_eq!(result.existing_count, 1);
}

#[test]
fn classify_all_new() {
    let baseline = Baseline { entries: vec![], version: "1.0.0".into() };
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
        ("src/main.rs".to_string(), "unused_variable".to_string(), 20),
    ];
    
    let result = BaselineClassifier::classify(&findings, &baseline);
    
    assert_eq!(result.new_count, 2);
    assert_eq!(result.existing_count, 0);
}

#[test]
fn classify_all_existing() {
    let baseline = create_baseline();
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
    ];
    
    let result = BaselineClassifier::classify(&findings, &baseline);
    
    assert_eq!(result.new_count, 0);
    assert_eq!(result.existing_count, 1);
}

#[test]
fn existing_violation_still_blocks() {
    let baseline = create_baseline();
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
    ];
    
    let result = BaselineClassifier::classify(&findings, &baseline);
    
    // 已存在违规仍应阻断（existing 不等于允许）
    assert_eq!(result.existing_count, 1);
    // 但分类为 Existing（不是 New）
    assert_eq!(result.findings[0].class, FindingClass::Existing);
}

#[test]
fn baseline_failure_does_not_eliminate_findings() {
    let baseline = Baseline { entries: vec![], version: "1.0.0".into() };
    
    // 基线验证失败
    assert!(BaselineClassifier::validate_baseline(&baseline).is_err());
    
    // 但 finding 仍存在
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
    ];
    let result = BaselineClassifier::classify(&findings, &baseline);
    assert_eq!(result.new_count, 1);
}

#[test]
fn validate_baseline_rejects_duplicates() {
    let baseline = Baseline {
        entries: vec![
            BaselineEntry {
                file: "a.rs".into(),
                rule: "rule".into(),
                line: 1,
                fingerprint: "a.rs:rule:1".into(),
            },
            BaselineEntry {
                file: "a.rs".into(),
                rule: "rule".into(),
                line: 1,
                fingerprint: "a.rs:rule:1".into(),
            },
        ],
        version: "1.0.0".into(),
    };
    
    assert!(BaselineClassifier::validate_baseline(&baseline).is_err());
}

#[test]
fn create_baseline_from_findings() {
    let findings = vec![
        ("src/main.rs".to_string(), "unused_variable".to_string(), 10),
        ("src/main.rs".to_string(), "unused_variable".to_string(), 20),
    ];
    
    let baseline = BaselineClassifier::create_baseline(&findings);
    assert_eq!(baseline.entries.len(), 2);
    
    // 验证基线
    assert!(BaselineClassifier::validate_baseline(&baseline).is_ok());
}
