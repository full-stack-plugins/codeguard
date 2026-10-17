//! 检查器配置探测测试

use codeguard_adapters::checker_config_detection::*;

#[test]
fn detect_configured_checker() {
    let configs = vec![
        ("javadoc".to_string(), "comments".to_string(), ConfigStatus::Configured),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    
    assert_eq!(result.checkers.len(), 1);
    assert_eq!(result.checkers[0].status, ConfigStatus::Configured);
    assert!(result.checkers[0].config_location.is_some());
}

#[test]
fn detect_missing_checker() {
    let configs = vec![
        ("checkstyle".to_string(), "lint".to_string(), ConfigStatus::Missing),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    
    assert_eq!(result.checkers[0].status, ConfigStatus::Missing);
    assert!(result.checkers[0].enable_suggestion.is_some());
}

#[test]
fn detect_invalid_checker() {
    let configs = vec![
        ("ruff".to_string(), "lint".to_string(), ConfigStatus::Invalid),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    
    assert_eq!(result.checkers[0].status, ConfigStatus::Invalid);
    assert!(result.checkers[0].enable_suggestion.is_some());
}

#[test]
fn detect_unknown_checker() {
    let configs = vec![
        ("unknown".to_string(), "security".to_string(), ConfigStatus::Unknown),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    
    assert_eq!(result.checkers[0].status, ConfigStatus::Unknown);
}

#[test]
fn validate_detection_result() {
    let configs = vec![
        ("javadoc".to_string(), "comments".to_string(), ConfigStatus::Configured),
        ("checkstyle".to_string(), "lint".to_string(), ConfigStatus::Missing),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    assert!(CheckerConfigDetector::validate(&result));
}

#[test]
fn detect_multiple_categories() {
    let configs = vec![
        ("javadoc".to_string(), "comments".to_string(), ConfigStatus::Configured),
        ("maven_dependency".to_string(), "dependencies".to_string(), ConfigStatus::Configured),
        ("owasp".to_string(), "cve".to_string(), ConfigStatus::Missing),
        ("spotbugs".to_string(), "security".to_string(), ConfigStatus::Configured),
    ];
    
    let result = CheckerConfigDetector::detect("/project", &configs);
    
    assert_eq!(result.checkers.len(), 4);
    assert!(CheckerConfigDetector::validate(&result));
}
