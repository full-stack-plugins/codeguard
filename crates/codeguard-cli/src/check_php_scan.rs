//! PHP lint 扫描适配器。
//!
//! 调用 PHP_CodeSniffer 原生工具进行 PHP 代码规范检查。
//! 配置状态区分 configured/missing/invalid/unknown。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

/// 观察 PHP lint 扫描结果。
pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "php_lint_scan",
        "language": "php",
        "config": config,
        "status": "incomplete",
        "reason": "php_native_tool_not_executed",
    })
}

/// 刷新报告（复用已有观察）。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let _ = (root, report, deadline);
}

/// 检查是否优先使用原生工具。
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool {
    report["status"] == "incomplete"
}

/// 适用性档案。
pub(crate) fn applicability_profile() -> Value {
    json!({
        "language": "php",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "phpcs"},
            {"category": "comments", "applicability": "applicable", "tool": "phpcs"},
        ],
    })
}

/// 观察规则配置状态。
pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "php_rule_config",
        "config": config,
    })
}

/// 复检规则配置。
pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

/// 观察 PHP 配置状态。
fn observe_config(root: &Path) -> Value {
    let phpcs_config = root.join(".phpcs.xml");
    let phpcs_config_alt = root.join("phpcs.xml");
    let composer = root.join("composer.json");

    let config = if phpcs_config.exists() || phpcs_config_alt.exists() {
        "configured"
    } else if composer.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if phpcs_config.exists() { ".phpcs.xml" } else if phpcs_config_alt.exists() { "phpcs.xml" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("php-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "php");
    }

    #[test]
    fn config_detection_identifies_phpcs_config() {
        let root = std::env::temp_dir().join("php-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("phpcs.xml"), "<ruleset/>").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        let cats = profile["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0]["category"], "lint");
        assert_eq!(cats[1]["category"], "comments");
    }
}
