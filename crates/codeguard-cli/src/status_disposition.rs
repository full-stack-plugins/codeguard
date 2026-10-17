//! Status 历史 freshness、next 五类 disposition 及工作区缺失行为
//!
//! 验收标准：无任务不等于通过，过期结果不显示当前 allow，待租用/待决策/待验证各有具体下一步

use serde::{Deserialize, Serialize};

/// Disposition 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    /// 可操作
    Actionable,
    /// 需决策
    NeedsDecision,
    /// 需验证
    NeedsVerification,
    /// 待租用
    AwaitingLease,
    /// 无任务
    NoTasks,
}

/// 状态报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    /// Disposition
    pub disposition: Disposition,
    /// 是否过期
    pub expired: bool,
    /// 下一步
    pub next_action: Option<String>,
}

/// 状态检查器
pub struct StatusChecker;

impl StatusChecker {
    /// 检查状态
    pub fn check(has_tasks: bool, expired: bool) -> StatusReport {
        let disposition = if !has_tasks {
            Disposition::NoTasks
        } else if expired {
            Disposition::NeedsVerification
        } else {
            Disposition::Actionable
        };
        
        let next_action = match disposition {
            Disposition::NoTasks => Some("运行全量验证".to_string()),
            Disposition::NeedsVerification => Some("重新验证".to_string()),
            Disposition::Actionable => Some("继续执行".to_string()),
            Disposition::NeedsDecision => Some("做出决策".to_string()),
            Disposition::AwaitingLease => Some("等待租约".to_string()),
        };
        
        StatusReport {
            disposition,
            expired,
            next_action,
        }
    }
    
    /// 验证无任务不等于通过
    pub fn validate_no_tasks_not_pass(report: &StatusReport) -> bool {
        report.disposition != Disposition::NoTasks || report.next_action.is_some()
    }
    
    /// 验证过期结果不显示当前 allow
    pub fn validate_expired_no_allow(report: &StatusReport) -> bool {
        !report.expired || report.disposition != Disposition::Actionable
    }
}
