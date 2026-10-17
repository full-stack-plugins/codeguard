//! 输入清单幂等刷新模块（9.23）。
//!
//! 实现输入清单与增删感知的幂等画像刷新：新增模块、锁/规则变化均失效，
//! 刷新保留 findings/events/备注。

use serde_json::{Value, json};
use std::path::Path;

/// 刷新状态。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RefreshStatus {
    /// 无变化。
    NoChange,
    /// 有变化。
    Changed,
    /// 首次刷新。
    Initial,
}

impl RefreshStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::NoChange => "no_change",
            Self::Changed => "changed",
            Self::Initial => "initial",
        }
    }
}

/// 刷新结果。
pub(crate) struct RefreshResult {
    pub status: RefreshStatus,
    pub new_modules: Vec<String>,
    pub removed_modules: Vec<String>,
    pub preserved_findings: u32,
}

/// 幂等刷新。
pub(crate) fn refresh_manifest(
    previous_modules: &[String],
    current_modules: &[String],
    findings_count: u32,
) -> RefreshResult {
    let new_modules: Vec<String> = current_modules
        .iter()
        .filter(|m| !previous_modules.contains(m))
        .cloned()
        .collect();
    let removed_modules: Vec<String> = previous_modules
        .iter()
        .filter(|m| !current_modules.contains(m))
        .cloned()
        .collect();

    let status = if previous_modules.is_empty() {
        RefreshStatus::Initial
    } else if new_modules.is_empty() && removed_modules.is_empty() {
        RefreshStatus::NoChange
    } else {
        RefreshStatus::Changed
    };

    RefreshResult {
        status,
        new_modules,
        removed_modules,
        preserved_findings: findings_count,
    }
}

/// 生成刷新报告。
pub(crate) fn refresh_report(result: &RefreshResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "manifest_refresh",
        "status": result.status.as_str(),
        "new_modules": result.new_modules,
        "removed_modules": result.removed_modules,
        "preserved_findings": result.preserved_findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_refresh_detected() {
        let result = refresh_manifest(&[], &["a".to_string()], 0);
        assert_eq!(result.status, RefreshStatus::Initial);
    }

    #[test]
    fn no_change_detected() {
        let result = refresh_manifest(&["a".to_string()], &["a".to_string()], 5);
        assert_eq!(result.status, RefreshStatus::NoChange);
        assert_eq!(result.preserved_findings, 5);
    }

    #[test]
    fn new_module_detected() {
        let result = refresh_manifest(&["a".to_string()], &["a".to_string(), "b".to_string()], 3);
        assert_eq!(result.status, RefreshStatus::Changed);
        assert_eq!(result.new_modules, vec!["b".to_string()]);
    }

    #[test]
    fn removed_module_detected() {
        let result = refresh_manifest(&["a".to_string(), "b".to_string()], &["a".to_string()], 3);
        assert_eq!(result.removed_modules, vec!["b".to_string()]);
    }

    #[test]
    fn findings_preserved_on_refresh() {
        let result = refresh_manifest(&["a".to_string()], &["a".to_string(), "b".to_string()], 10);
        assert_eq!(result.preserved_findings, 10);
    }
}
