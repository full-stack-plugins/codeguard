//! 报告 freshness 测试

use codeguard_adapters::report_freshness::*;

fn create_metadata() -> ReportMetadata {
    ReportMetadata {
        id: "report-1".into(),
        created_at: 100,
        expires_at: 200,
        rules_covered: vec!["rule1".into(), "rule2".into()],
        scope_covered: vec!["file1.rs".into()],
    }
}

#[test]
fn fresh_report_passes() {
    let metadata = create_metadata();
    let result = ReportFreshnessChecker::check(
        &metadata,
        150,
        &["rule1".to_string(), "rule2".to_string()],
        &["file1.rs".to_string()],
    );
    
    assert!(result.passed);
    assert_eq!(result.status, ReportStatus::Fresh);
}

#[test]
fn stale_report_fails() {
    let metadata = create_metadata();
    let result = ReportFreshnessChecker::check(
        &metadata,
        250,
        &["rule1".to_string()],
        &["file1.rs".to_string()],
    );
    
    assert!(!result.passed);
    assert_eq!(result.status, ReportStatus::Stale);
}

#[test]
fn forged_empty_report_fails() {
    let metadata = ReportMetadata {
        id: "report-1".into(),
        created_at: 100,
        expires_at: 200,
        rules_covered: vec![],
        scope_covered: vec![],
    };
    
    let result = ReportFreshnessChecker::check(&metadata, 150, &[], &[]);
    assert!(!result.passed);
    assert_eq!(result.status, ReportStatus::Forged);
}

#[test]
fn incomplete_coverage_fails() {
    let metadata = create_metadata();
    let result = ReportFreshnessChecker::check(
        &metadata,
        150,
        &["rule1".to_string(), "rule2".to_string(), "rule3".to_string()],
        &["file1.rs".to_string()],
    );
    
    assert!(!result.passed);
}

#[test]
fn validate_not_forged() {
    let metadata = create_metadata();
    assert!(ReportFreshnessChecker::validate_not_forged(&metadata));
    
    let empty = ReportMetadata {
        id: "report-1".into(),
        created_at: 100,
        expires_at: 200,
        rules_covered: vec![],
        scope_covered: vec![],
    };
    assert!(!ReportFreshnessChecker::validate_not_forged(&empty));
}
