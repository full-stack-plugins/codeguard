//! r lint 扫描适配器。
//!
//! 调用 lintr 原生工具进行 r 代码规范检查。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "r_lint_scan",
        "language": "r",
        "config": config,
        "status": "incomplete",
        "reason": "r_native_tool_not_executed",
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
        "language": "r",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "lintr"},
            {"category": "comments", "applicability": "applicable", "tool": "lintr"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join(".lintr");
    let fallback = root.join("DESCRIPTION");

    let config = if config_file.exists() {
        "configured"
    } else if fallback.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if config_file.exists() { ".lintr" } else if fallback.exists() { "DESCRIPTION" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("r-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "r");
    }

    #[test]
    fn config_detection_identifies_config() {
        let root = std::env::temp_dir().join("r-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".lintr"), "").unwrap();
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
