//! 报告产物 freshness：陈旧或伪造空成功报告不能通过
//!
//! 验收标准：scope/rule 执行覆盖核对

use serde::{Deserialize, Serialize};

/// 报告状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
    /// 新鲜
    Fresh,
    /// 陈旧
    Stale,
    /// 伪造
    Forged,
}

/// 报告元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// 报告 ID
    pub id: String,
    /// 创建时间
    pub created_at: u64,
    /// 过期时间
    pub expires_at: u64,
    /// 覆盖的规则
    pub rules_covered: Vec<String>,
    /// 覆盖的范围
    pub scope_covered: Vec<String>,
}

/// Freshness 检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessResult {
    /// 报告状态
    pub status: ReportStatus,
    /// 是否通过
    pub passed: bool,
    /// 原因
    pub reason: Option<String>,
}

/// 报告 freshness 检查器
pub struct ReportFreshnessChecker;

impl ReportFreshnessChecker {
    /// 检查报告 freshness
    pub fn check(
        metadata: &ReportMetadata,
        current_time: u64,
        expected_rules: &[String],
        expected_scope: &[String],
    ) -> FreshnessResult {
        // 检查是否陈旧
        if current_time > metadata.expires_at {
            return FreshnessResult {
                status: ReportStatus::Stale,
                passed: false,
                reason: Some("report_expired".into()),
            };
        }
        
        // 检查是否伪造空成功
        if metadata.rules_covered.is_empty() && metadata.scope_covered.is_empty() {
            return FreshnessResult {
                status: ReportStatus::Forged,
                passed: false,
                reason: Some("empty_success_forged".into()),
            };
        }
        
        // 检查覆盖范围
        let rules_covered = expected_rules.iter()
            .all(|r| metadata.rules_covered.contains(r));
        let scope_covered = expected_scope.iter()
            .all(|s| metadata.scope_covered.contains(s));
        
        if !rules_covered || !scope_covered {
            return FreshnessResult {
                status: ReportStatus::Stale,
                passed: false,
                reason: Some("incomplete_coverage".into()),
            };
        }
        
        FreshnessResult {
            status: ReportStatus::Fresh,
            passed: true,
            reason: None,
        }
    }
    
    /// 验证报告不伪造
    pub fn validate_not_forged(metadata: &ReportMetadata) -> bool {
        !metadata.rules_covered.is_empty() || !metadata.scope_covered.is_empty()
    }
}
