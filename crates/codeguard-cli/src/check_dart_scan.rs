//! dart lint 扫描适配器。
//!
//! 调用 dart_analyze 原生工具进行 dart 代码规范检查。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "dart_lint_scan",
        "language": "dart",
        "config": config,
        "status": "incomplete",
        "reason": "dart_native_tool_not_executed",
    })
}

pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let _ = (root, report, deadline);
}

pub(crate) fn prefers(report: &Value, _relative: &str) -> bool {
    report["status"] == "incomplete"
}

pub(crate) fn applicability_profile() -> Value {
    json!({
        "language": "dart",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "dart_analyze"},
            {"category": "comments", "applicability": "applicable", "tool": "dart_analyze"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join("analysis_options.yaml");
    let fallback = root.join("pubspec.yaml");

    let config = if config_file.exists() {
        "configured"
    } else if fallback.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if config_file.exists() { "analysis_options.yaml" } else if fallback.exists() { "pubspec.yaml" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("dart-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "dart");
    }

    #[test]
    fn config_detection_identifies_config() {
        let root = std::env::temp_dir().join("dart-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("analysis_options.yaml"), "").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        let cats = profile["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 2);
    }
}
