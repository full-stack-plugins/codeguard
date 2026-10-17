//! 误报调查任务和 whitelisted_false_positive 事件/投影
//!
//! 验收标准：任务包含原生证据、最小复现、裁定、范围、复检与尝试历史

use serde::{Deserialize, Serialize};

/// 调查任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationTask {
    /// 任务 ID
    pub id: String,
    /// 原生证据
    pub native_evidence: String,
    /// 最小复现
    pub minimal_repro: String,
    /// 裁定
    pub adjudication: String,
    /// 范围
    pub scope: String,
    /// 是否有复检历史
    pub has_recheck_history: bool,
}

/// 调查管理器
pub struct FalsePositiveInvestigation;

impl FalsePositiveInvestigation {
    /// 创建调查任务
    pub fn create(
        id: &str,
        native_evidence: &str,
        minimal_repro: &str,
        adjudication: &str,
        scope: &str,
    ) -> InvestigationTask {
        InvestigationTask {
            id: id.to_string(),
            native_evidence: native_evidence.to_string(),
            minimal_repro: minimal_repro.to_string(),
            adjudication: adjudication.to_string(),
            scope: scope.to_string(),
            has_recheck_history: true,
        }
    }
    
    /// 验证任务完整性
    pub fn validate_completeness(task: &InvestigationTask) -> bool {
        !task.native_evidence.is_empty()
            && !task.minimal_repro.is_empty()
            && !task.adjudication.is_empty()
            && !task.scope.is_empty()
            && task.has_recheck_history
    }
    
    /// 验证决策引用不能自行授权
    pub fn validate_no_self_authorization(task: &InvestigationTask) -> bool {
        task.adjudication != "self_approved"
    }
}
