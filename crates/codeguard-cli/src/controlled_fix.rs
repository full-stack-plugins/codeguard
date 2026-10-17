//! 受控 fix 接口。
//!
//! 提供 attempt 与验证事件接口；noop、部分修改失败及并发编辑的事件样本分别记录
//! 且不生成假修复；真实 formatter 接线在 S10 完成。

use serde_json::{Value, json};

/// attempt 事件类型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AttemptEvent {
    /// noop：无修改。
    Noop,
    /// 部分修改失败。
    PartialFailure,
    /// 并发编辑冲突。
    ConcurrentEdit,
    /// 成功修改。
    Success,
}

impl AttemptEvent {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::PartialFailure => "partial_failure",
            Self::ConcurrentEdit => "concurrent_edit",
            Self::Success => "success",
        }
    }
}

/// 验证事件类型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VerificationEvent {
    /// 问题仍存在。
    StillPresent,
    /// 问题已解决。
    Resolved,
    /// 验证未完成（工具缺失等）。
    Incomplete,
    /// 问题变化（复发）。
    Recurrence,
}

impl VerificationEvent {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::StillPresent => "still_present",
            Self::Resolved => "resolved",
            Self::Incomplete => "incomplete",
            Self::Recurrence => "recurrence",
        }
    }
}

/// attempt 事件记录。
pub(crate) struct AttemptRecord {
    pub task_id: String,
    pub event: AttemptEvent,
    pub timestamp: String,
    pub detail: String,
}

/// 验证事件记录。
pub(crate) struct VerificationRecord {
    pub task_id: String,
    pub event: VerificationEvent,
    pub timestamp: String,
    pub detail: String,
}

/// 记录 attempt 事件。
pub(crate) fn record_attempt(task_id: &str, event: AttemptEvent, detail: &str) -> AttemptRecord {
    AttemptRecord {
        task_id: task_id.to_string(),
        event,
        timestamp: chrono_like_timestamp(),
        detail: detail.to_string(),
    }
}

/// 记录验证事件。
pub(crate) fn record_verification(
    task_id: &str,
    event: VerificationEvent,
    detail: &str,
) -> VerificationRecord {
    VerificationRecord {
        task_id: task_id.to_string(),
        event,
        timestamp: chrono_like_timestamp(),
        detail: detail.to_string(),
    }
}

/// 生成 attempt 事件报告。
pub(crate) fn attempt_report(records: &[AttemptRecord]) -> Value {
    let mut noop = 0;
    let mut partial_failure = 0;
    let mut concurrent_edit = 0;
    let mut success = 0;

    for record in records {
        match record.event {
            AttemptEvent::Noop => noop += 1,
            AttemptEvent::PartialFailure => partial_failure += 1,
            AttemptEvent::ConcurrentEdit => concurrent_edit += 1,
            AttemptEvent::Success => success += 1,
        }
    }

    json!({
        "schema_version": "0.1.0",
        "report_type": "attempt_report",
        "noop": noop,
        "partial_failure": partial_failure,
        "concurrent_edit": concurrent_edit,
        "success": success,
    })
}

/// 生成验证事件报告。
pub(crate) fn verification_report(records: &[VerificationRecord]) -> Value {
    let mut still_present = 0;
    let mut resolved = 0;
    let mut incomplete = 0;
    let mut recurrence = 0;

    for record in records {
        match record.event {
            VerificationEvent::StillPresent => still_present += 1,
            VerificationEvent::Resolved => resolved += 1,
            VerificationEvent::Incomplete => incomplete += 1,
            VerificationEvent::Recurrence => recurrence += 1,
        }
    }

    json!({
        "schema_version": "0.1.0",
        "report_type": "verification_report",
        "still_present": still_present,
        "resolved": resolved,
        "incomplete": incomplete,
        "recurrence": recurrence,
    })
}

/// 生成模拟时间戳（测试用）。
fn chrono_like_timestamp() -> String {
    "2026-10-07T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_attempt_noop() {
        let record = record_attempt("task-1", AttemptEvent::Noop, "no changes");
        assert_eq!(record.event, AttemptEvent::Noop);
        assert_eq!(record.event.as_str(), "noop");
    }

    #[test]
    fn record_attempt_partial_failure() {
        let record = record_attempt("task-1", AttemptEvent::PartialFailure, "file locked");
        assert_eq!(record.event, AttemptEvent::PartialFailure);
    }

    #[test]
    fn record_attempt_concurrent_edit() {
        let record = record_attempt("task-1", AttemptEvent::ConcurrentEdit, "file changed");
        assert_eq!(record.event, AttemptEvent::ConcurrentEdit);
    }

    #[test]
    fn record_verification_resolved() {
        let record = record_verification("task-1", VerificationEvent::Resolved, "fixed");
        assert_eq!(record.event, VerificationEvent::Resolved);
    }

    #[test]
    fn record_verification_recurrence() {
        let record = record_verification("task-1", VerificationEvent::Recurrence, "reappeared");
        assert_eq!(record.event, VerificationEvent::Recurrence);
    }

    #[test]
    fn attempt_report_counts_all_events() {
        let records = vec![
            record_attempt("t1", AttemptEvent::Noop, ""),
            record_attempt("t1", AttemptEvent::PartialFailure, ""),
            record_attempt("t1", AttemptEvent::ConcurrentEdit, ""),
            record_attempt("t1", AttemptEvent::Success, ""),
        ];
        let report = attempt_report(&records);
        assert_eq!(report["noop"], 1);
        assert_eq!(report["partial_failure"], 1);
        assert_eq!(report["concurrent_edit"], 1);
        assert_eq!(report["success"], 1);
    }

    #[test]
    fn verification_report_counts_all_events() {
        let records = vec![
            record_verification("t1", VerificationEvent::StillPresent, ""),
            record_verification("t1", VerificationEvent::Resolved, ""),
            record_verification("t1", VerificationEvent::Incomplete, ""),
            record_verification("t1", VerificationEvent::Recurrence, ""),
        ];
        let report = verification_report(&records);
        assert_eq!(report["still_present"], 1);
        assert_eq!(report["resolved"], 1);
        assert_eq!(report["incomplete"], 1);
        assert_eq!(report["recurrence"], 1);
    }

    #[test]
    fn noop_does_not_generate_fake_fix() {
        let record = record_attempt("task-1", AttemptEvent::Noop, "no changes");
        assert_eq!(record.event, AttemptEvent::Noop);
        // noop 不是成功修复
        assert_ne!(record.event, AttemptEvent::Success);
    }
}
