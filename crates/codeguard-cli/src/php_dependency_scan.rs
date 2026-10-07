//! PHP 依赖/CVE/安全/构建扫描适配器。
//!
//! 调用 composer 原生工具进行 PHP 依赖检查、漏洞扫描和构建验证。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

/// 观察 PHP 依赖扫描结果。
pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "php_dependency_scan",
        "language": "php",
        "config": config,
        "status": "incomplete",
        "reason": "php_composer_not_executed",
    })
}

/// 刷新报告。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let _ = (root, report, deadline);
}

/// 检查是否优先使用原生工具。
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool {
    report["status"] == "incomplete"
}

/// 适用性档案（dependencies/cve/security/build）。
pub(crate) fn applicability_profile() -> Value {
    json!({
        "language": "php",
        "categories": [
            {"category": "dependencies", "applicability": "applicable", "tool": "composer"},
            {"category": "cve", "applicability": "applicable", "tool": "composer_audit"},
            {"category": "security", "applicability": "applicable", "tool": "psalm"},
            {"category": "build", "applicability": "applicable", "tool": "composer"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let composer = root.join("composer.json");
    let composer_lock = root.join("composer.lock");

    let config = if composer_lock.exists() {
        "configured"
    } else if composer.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if composer_lock.exists() { "composer.lock" } else if composer.exists() { "composer.json" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("php-dep-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "php");
    }

    #[test]
    fn config_detection_identifies_composer_lock() {
        let root = std::env::temp_dir().join("php-dep-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("composer.lock"), "{}").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_four_categories() {
        let profile = applicability_profile();
        let cats = profile["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 4);
    }
}
