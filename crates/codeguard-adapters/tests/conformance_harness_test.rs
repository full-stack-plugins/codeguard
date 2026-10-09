//! Conformance harness 测试

use codeguard_adapters::conformance_harness::*;

/// 创建 F01 用例
fn f01_case() -> ConformanceCase {
    ConformanceCase {
        scenario: ScenarioId::F01,
        id: "f01_valid".into(),
        description: "正确源码，正确配置与版本".into(),
        evidence_tier: EvidenceTier::Mock,
        valid_report: Some(b"{}".to_vec()),
        malformed_report: None,
        expected: ExpectedResult {
            should_pass: true,
            expected_findings: Some(0),
            expected_incomplete_reason: None,
        },
    }
}

/// 创建 F03 用例
fn f03_case() -> ConformanceCase {
    ConformanceCase {
        scenario: ScenarioId::F03,
        id: "f03_missing_tool".into(),
        description: "缺命令/运行时".into(),
        evidence_tier: EvidenceTier::Mock,
        valid_report: None,
        malformed_report: Some(b"invalid".to_vec()),
        expected: ExpectedResult {
            should_pass: false,
            expected_findings: None,
            expected_incomplete_reason: Some("tool_not_found".into()),
        },
    }
}

/// 创建 F06 用例
fn f06_case() -> ConformanceCase {
    ConformanceCase {
        scenario: ScenarioId::F06,
        id: "f06_empty_report".into(),
        description: "exit 0 但空/畸形报告".into(),
        evidence_tier: EvidenceTier::Mock,
        valid_report: Some(b"".to_vec()),
        malformed_report: None,
        expected: ExpectedResult {
            should_pass: false,
            expected_findings: None,
            expected_incomplete_reason: Some("empty_report".into()),
        },
    }
}

#[test]
fn harness_add_and_filter_cases() {
    let mut harness = ConformanceHarness::new();
    harness.add_case(f01_case());
    harness.add_case(f03_case());
    harness.add_case(f06_case());
    
    let f01 = harness.filter_by_scenario(ScenarioId::F01);
    assert_eq!(f01.len(), 1);
    
    let mock = harness.filter_by_tier(EvidenceTier::Mock);
    assert_eq!(mock.len(), 3);
}

#[test]
fn harness_run_valid_report() {
    let mut harness = ConformanceHarness::new();
    harness.add_case(f01_case());
    
    let parser = |_: &[u8]| -> Result<(usize, Option<String>), String> {
        Ok((0, None))
    };
    
    let results = harness.run(&parser);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed);
}

#[test]
fn harness_run_malformed_report() {
    let mut harness = ConformanceHarness::new();
    harness.add_case(f03_case());
    
    let parser = |_: &[u8]| -> Result<(usize, Option<String>), String> {
        Ok((0, Some("tool_not_found".into())))
    };
    
    let results = harness.run(&parser);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed);
}

#[test]
fn harness_evidence_tier_separation() {
    let mut harness = ConformanceHarness::new();
    
    let mut mock_case = f01_case();
    mock_case.evidence_tier = EvidenceTier::Mock;
    harness.add_case(mock_case);
    
    let mut real_case = f01_case();
    real_case.id = "f01_real".into();
    real_case.evidence_tier = EvidenceTier::Real;
    harness.add_case(real_case);
    
    let mock = harness.filter_by_tier(EvidenceTier::Mock);
    let real = harness.filter_by_tier(EvidenceTier::Real);
    
    assert_eq!(mock.len(), 1);
    assert_eq!(real.len(), 1);
}

#[test]
fn harness_summarize_results() {
    let mut harness = ConformanceHarness::new();
    harness.add_case(f01_case());
    harness.add_case(f03_case());
    
    let parser = |report: &[u8]| -> Result<(usize, Option<String>), String> {
        if report.is_empty() {
            Ok((0, Some("empty".into())))
        } else {
            Ok((0, None))
        }
    };
    
    let results = harness.run(&parser);
    let summary = ConformanceHarness::summarize(&results);
    
    assert_eq!(summary.total, 2);
    assert_eq!(summary.mock_cases, 2);
}

#[test]
fn harness_f01_f10_f17_coverage() {
    let mut harness = ConformanceHarness::new();
    
    // 添加 F01-F10 和 F17 用例
    let scenarios = vec![
        ScenarioId::F01, ScenarioId::F02, ScenarioId::F03,
        ScenarioId::F04, ScenarioId::F05, ScenarioId::F06,
        ScenarioId::F07, ScenarioId::F08, ScenarioId::F09,
        ScenarioId::F10, ScenarioId::F17,
    ];
    
    for scenario in scenarios {
        harness.add_case(ConformanceCase {
            scenario,
            id: format!("{:?}", scenario),
            description: "test".into(),
            evidence_tier: EvidenceTier::Mock,
            valid_report: Some(b"{}".to_vec()),
            malformed_report: None,
            expected: ExpectedResult {
                should_pass: true,
                expected_findings: Some(0),
                expected_incomplete_reason: None,
            },
        });
    }
    
    // 验证 F01-F10 和 F17 都有覆盖
    for scenario in [ScenarioId::F01, ScenarioId::F05, ScenarioId::F10, ScenarioId::F17] {
        assert!(!harness.filter_by_scenario(scenario).is_empty());
    }
}
