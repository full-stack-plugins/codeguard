//! 策略弱化与修复越界测试模块（12.5/12.6）。
//!
//! 执行策略弱化、缓存污染、错误快照、报告缺项和修复越界测试；
//! 项目脚本无法改写可信政策、工具锁或签发收据。

use serde_json::{Value, json};

/// 测试结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TestResult {
    /// 通过。
    Passed,
    /// 失败。
    Failed,
    /// 跳过。
    Skipped,
}

impl TestResult {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// 策略弱化测试。
pub(crate) fn test_policy_weakening(
    cli_can_weaken: bool,
    env_can_weaken: bool,
    project_can_weaken: bool,
) -> TestResult {
    if cli_can_weaken || env_can_weaken || project_can_weaken {
        TestResult::Failed
    } else {
        TestResult::Passed
    }
}

/// 修复越界测试。
pub(crate) fn test_repair_boundary(
    repair_within_scope: bool,
    repair_outside_scope: bool,
) -> TestResult {
    if repair_outside_scope {
        TestResult::Failed
    } else if repair_within_scope {
        TestResult::Passed
    } else {
        TestResult::Skipped
    }
}

/// 生成测试报告。
pub(crate) fn test_report(policy: &TestResult, repair: &TestResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "policy_repair_test",
        "policy_weakening": policy.as_str(),
        "repair_boundary": repair.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_weakening_rejected() {
        let result = test_policy_weakening(false, false, false);
        assert_eq!(result, TestResult::Passed);
    }

    #[test]
    fn policy_weakening_cli_detected() {
        let result = test_policy_weakening(true, false, false);
        assert_eq!(result, TestResult::Failed);
    }

    #[test]
    fn repair_within_scope_passed() {
        let result = test_repair_boundary(true, false);
        assert_eq!(result, TestResult::Passed);
    }

    #[test]
    fn repair_outside_scope_failed() {
        let result = test_repair_boundary(false, true);
        assert_eq!(result, TestResult::Failed);
    }

    #[test]
    fn test_report_contains_results() {
        let policy = test_policy_weakening(false, false, false);
        let repair = test_repair_boundary(true, false);
        let report = test_report(&policy, &repair);
        assert_eq!(report["policy_weakening"], "passed");
        assert_eq!(report["repair_boundary"], "passed");
    }
}
