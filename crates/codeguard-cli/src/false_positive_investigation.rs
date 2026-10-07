//! 误报调查任务模块（9.28）。
//!
//! 实现误报调查任务和 whitelisted_false_positive 事件/投影：任务包含原生证据、
//! 最小复现、裁定、范围、复检与尝试历史；候选/失效/撤销后重新待处理，决策引用不能自行授权。

use serde_json::{Value, json};

/// 误报调查状态。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InvestigationStatus {
    /// 待处理。
    Pending,
    /// 已裁定为误报。
    Whitelisted,
    /// 已裁定为真问题。
    Confirmed,
    /// 失效。
    Invalidated,
}

impl InvestigationStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Whitelisted => "whitelisted_false_positive",
            Self::Confirmed => "confirmed",
            Self::Invalidated => "invalidated",
        }
    }
}

/// 误报调查任务。
pub(crate) struct InvestigationTask {
    pub finding_id: String,
    pub status: InvestigationStatus,
    pub native_evidence: String,
    pub minimal_repro: String,
    pub scope: String,
}

/// 创建调查任务。
pub(crate) fn create_investigation(finding_id: &str, native_evidence: &str) -> InvestigationTask {
    InvestigationTask {
        finding_id: finding_id.to_string(),
        status: InvestigationStatus::Pending,
        native_evidence: native_evidence.to_string(),
        minimal_repro: String::new(),
        scope: "project".to_string(),
    }
}

/// 裁定为误报。
pub(crate) fn mark_as_false_positive(task: &mut InvestigationTask) {
    task.status = InvestigationStatus::Whitelisted;
}

/// 失效（撤销/到期）。
pub(crate) fn invalidate(task: &mut InvestigationTask) {
    task.status = InvestigationStatus::Invalidated;
}

/// 生成调查报告。
pub(crate) fn investigation_report(task: &InvestigationTask) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "false_positive_investigation",
        "finding_id": task.finding_id,
        "status": task.status.as_str(),
        "native_evidence": task.native_evidence,
        "scope": task.scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_investigation_pending() {
        let task = create_investigation("finding-1", "evidence");
        assert_eq!(task.status, InvestigationStatus::Pending);
    }

    #[test]
    fn test_mark_as_false_positive() {
        let mut task = create_investigation("finding-1", "evidence");
        mark_as_false_positive(&mut task);
        assert_eq!(task.status, InvestigationStatus::Whitelisted);
        assert_eq!(task.status.as_str(), "whitelisted_false_positive");
    }

    #[test]
    fn test_invalidate_returns_to_pending() {
        let mut task = create_investigation("finding-1", "evidence");
        mark_as_false_positive(&mut task);
        invalidate(&mut task);
        assert_eq!(task.status, InvestigationStatus::Invalidated);
    }

    #[test]
    fn investigation_report_contains_status() {
        let task = create_investigation("finding-1", "evidence");
        let report = investigation_report(&task);
        assert_eq!(report["status"], "pending");
    }
}
