//! Conformance harness：F01-F10/F17 有效与畸形报告可独立复用
//!
//! 验收标准：mock 与真工具证据分层

use serde::{Deserialize, Serialize};

/// 场景 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScenarioId {
    /// F01: 正确源码，正确配置与版本
    F01,
    /// F02: 一条确定违规与对应最小修复
    F02,
    /// F03: 缺命令/运行时、版本不兼容、坏配置
    F03,
    /// F04: 原生非零但没有有效报告
    F04,
    /// F05: 有效违规后超时/崩溃/输出截断
    F05,
    /// F06: exit 0 但空/畸形/陈旧/矛盾报告
    F06,
    /// F07: 模块/方言/生成代码/配置继承
    F07,
    /// F08: 字符编码、特殊路径、空格、换行、非 UTF-8
    F08,
    /// F09: 内容或原生配置同大小同 mtime 替换
    F09,
    /// F10: 规则/工具/漏洞库更换
    F10,
    /// F17: 证据保留与追溯
    F17,
}

/// 测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceCase {
    /// 场景 ID
    pub scenario: ScenarioId,
    /// 用例 ID
    pub id: String,
    /// 描述
    pub description: String,
    /// 证据层级（mock/real）
    pub evidence_tier: EvidenceTier,
    /// 有效报告（正例）
    pub valid_report: Option<Vec<u8>>,
    /// 畸形报告（反例）
    pub malformed_report: Option<Vec<u8>>,
    /// 期望结果
    pub expected: ExpectedResult,
}

/// 证据层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceTier {
    /// Mock 证据
    Mock,
    /// 真实工具证据
    Real,
}

/// 期望结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedResult {
    /// 是否应通过
    pub should_pass: bool,
    /// 期望发现数
    pub expected_findings: Option<usize>,
    /// 期望未完成原因
    pub expected_incomplete_reason: Option<String>,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceResult {
    /// 用例 ID
    pub case_id: String,
    /// 是否通过
    pub passed: bool,
    /// 实际发现数
    pub actual_findings: usize,
    /// 实际未完成原因
    pub actual_incomplete_reason: Option<String>,
    /// 证据层级
    pub evidence_tier: EvidenceTier,
}

/// Conformance harness
pub struct ConformanceHarness {
    cases: Vec<ConformanceCase>,
}

impl ConformanceHarness {
    /// 创建空 harness
    pub fn new() -> Self {
        Self { cases: Vec::new() }
    }
    
    /// 添加用例
    pub fn add_case(&mut self, case: ConformanceCase) {
        self.cases.push(case);
    }
    
    /// 按场景过滤
    pub fn filter_by_scenario(&self, scenario: ScenarioId) -> Vec<&ConformanceCase> {
        self.cases.iter().filter(|c| c.scenario == scenario).collect()
    }
    
    /// 按证据层级过滤
    pub fn filter_by_tier(&self, tier: EvidenceTier) -> Vec<&ConformanceCase> {
        self.cases.iter().filter(|c| c.evidence_tier == tier).collect()
    }
    
    /// 运行测试
    pub fn run(&self, parser: &dyn Fn(&[u8]) -> Result<(usize, Option<String>), String>) -> Vec<ConformanceResult> {
        self.cases.iter().map(|case| {
            let report = case.valid_report.as_deref().unwrap_or(&[]);
            let (findings, reason) = parser(report).unwrap_or((0, Some("parse_error".into())));
            
            let passed = if case.expected.should_pass {
                findings == case.expected.expected_findings.unwrap_or(findings)
                    && reason.is_none()
            } else {
                reason.is_some()
            };
            
            ConformanceResult {
                case_id: case.id.clone(),
                passed,
                actual_findings: findings,
                actual_incomplete_reason: reason,
                evidence_tier: case.evidence_tier,
            }
        }).collect()
    }
    
    /// 统计结果
    pub fn summarize(results: &[ConformanceResult]) -> ConformanceSummary {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let mock = results.iter().filter(|r| r.evidence_tier == EvidenceTier::Mock).count();
        let real = results.iter().filter(|r| r.evidence_tier == EvidenceTier::Real).count();
        
        ConformanceSummary {
            total,
            passed,
            failed: total - passed,
            mock_cases: mock,
            real_cases: real,
        }
    }
}

impl Default for ConformanceHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// 汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceSummary {
    /// 总数
    pub total: usize,
    /// 通过数
    pub passed: usize,
    /// 失败数
    pub failed: usize,
    /// Mock 用例数
    pub mock_cases: usize,
    /// 真实工具用例数
    pub real_cases: usize,
}
