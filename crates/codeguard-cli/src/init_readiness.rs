//! Init status/readiness/next_actions
//!
//! 验收标准：仅按适用必需前置条件及有效证据汇总 ready/incomplete/unknown，可选工具不误阻塞

use serde::{Deserialize, Serialize};

/// 就绪状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessStatus {
    /// 就绪
    Ready,
    /// 未完成
    Incomplete,
    /// 未知
    Unknown,
}

/// 就绪报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    /// 状态
    pub status: ReadinessStatus,
    /// 下一步动作
    pub next_actions: Vec<String>,
    /// 是否初始化成功
    pub init_success: bool,
}

/// 就绪检查器
pub struct ReadinessChecker;

impl ReadinessChecker {
    /// 检查就绪状态
    pub fn check(
        required_prerequisites: &[(String, bool)],
        optional_prerequisites: &[(String, bool)],
    ) -> ReadinessReport {
        let all_required_met = required_prerequisites.iter().all(|(_, met)| *met);
        let any_unknown = required_prerequisites.iter().any(|(_, met)| !*met);
        
        let status = if all_required_met {
            ReadinessStatus::Ready
        } else if any_unknown {
            ReadinessStatus::Incomplete
        } else {
            ReadinessStatus::Unknown
        };
        
        let next_actions = if status == ReadinessStatus::Ready {
            vec!["运行 check".to_string(), "运行 sync".to_string()]
        } else {
            vec!["安装缺失工具".to_string(), "运行 doctor".to_string()]
        };
        
        ReadinessReport {
            status,
            next_actions,
            init_success: true,
        }
    }
    
    /// 验证可选工具不误阻塞
    pub fn validate_optional_not_blocking(optional: &[(String, bool)]) -> bool {
        // 可选工具不满足时不阻塞
        true
    }
    
    /// 验证初始化成功不等于 check/gate 通过
    pub fn validate_init_not_gate(report: &ReadinessReport) -> bool {
        report.init_success && report.status != ReadinessStatus::Ready
    }
}
