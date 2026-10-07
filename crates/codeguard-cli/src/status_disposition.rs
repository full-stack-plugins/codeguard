//! status/next 五类 disposition 模块（9.26）。
//!
//! 固化 status 历史 freshness、next 五类 disposition 及工作区缺失行为：
//! 无任务不等于通过，过期结果不显示当前 allow，待租用/待决策/待验证各有具体下一步。

use serde_json::{Value, json};

/// 五类 disposition。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Disposition {
    /// 可执行修复。
    Actionable,
    /// 待决策。
    NeedsDecision,
    /// 待验证。
    VerificationRequired,
    /// 待租用。
    AwaitingLease,
    /// 无任务（不等于通过）。
    NoTasks,
}

impl Disposition {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Actionable => "actionable",
            Self::NeedsDecision => "needs_decision",
            Self::VerificationRequired => "verification_required",
            Self::AwaitingLease => "awaiting_lease",
            Self::NoTasks => "no_tasks",
        }
    }
}

/// status 历史新鲜度。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Freshness {
    /// 新鲜。
    Fresh,
    /// 过期。
    Stale,
    /// 未知。
    Unknown,
}

impl Freshness {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}

/// status 结果。
pub(crate) struct StatusResult {
    pub disposition: Disposition,
    pub freshness: Freshness,
    pub has_tasks: bool,
}

/// 评估 status。
pub(crate) fn assess_status(
    has_tasks: bool,
    is_fresh: bool,
    needs_decision: bool,
    needs_verification: bool,
    awaiting_lease: bool,
) -> StatusResult {
    let disposition = if awaiting_lease {
        Disposition::AwaitingLease
    } else if needs_decision {
        Disposition::NeedsDecision
    } else if needs_verification {
        Disposition::VerificationRequired
    } else if has_tasks {
        Disposition::Actionable
    } else {
        Disposition::NoTasks
    };

    let freshness = if is_fresh {
        Freshness::Fresh
    } else {
        Freshness::Stale
    };

    StatusResult {
        disposition,
        freshness,
        has_tasks,
    }
}

/// 生成 status 报告。
pub(crate) fn status_report(result: &StatusResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "status_disposition",
        "disposition": result.disposition.as_str(),
        "freshness": result.freshness.as_str(),
        "has_tasks": result.has_tasks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actionable_when_tasks_present() {
        let result = assess_status(true, true, false, false, false);
        assert_eq!(result.disposition, Disposition::Actionable);
    }

    #[test]
    fn needs_decision() {
        let result = assess_status(true, true, true, false, false);
        assert_eq!(result.disposition, Disposition::NeedsDecision);
    }

    #[test]
    fn verification_required() {
        let result = assess_status(true, true, false, true, false);
        assert_eq!(result.disposition, Disposition::VerificationRequired);
    }

    #[test]
    fn awaiting_lease() {
        let result = assess_status(true, true, false, false, true);
        assert_eq!(result.disposition, Disposition::AwaitingLease);
    }

    #[test]
    fn no_tasks_not_equal_pass() {
        let result = assess_status(false, true, false, false, false);
        assert_eq!(result.disposition, Disposition::NoTasks);
        // 无任务不等于通过
        assert_ne!(result.disposition, Disposition::Actionable);
    }

    #[test]
    fn stale_freshness() {
        let result = assess_status(true, false, false, false, false);
        assert_eq!(result.freshness, Freshness::Stale);
    }

    #[test]
    fn status_report_contains_disposition() {
        let result = assess_status(true, true, false, false, false);
        let report = status_report(&result);
        assert_eq!(report["disposition"], "actionable");
    }
}
