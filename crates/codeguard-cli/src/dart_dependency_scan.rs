//! dart 依赖/CVE/安全/构建扫描适配器。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "dart_dependency_scan",
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
            {"category": "dependencies", "applicability": "applicable", "tool": "dart_pub"},
            {"category": "cve", "applicability": "applicable", "tool": "dart_pub"},
            {"category": "security", "applicability": "applicable", "tool": "dart_pub"},
            {"category": "build", "applicability": "applicable", "tool": "dart_pub"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join("pubspec.yaml");
    let config = if config_file.exists() { "configured" } else { "unknown" };
    json!({
        "status": config,
        "config_ref": if config_file.exists() { "pubspec.yaml" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("dart-dep-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
    }

    #[test]
    fn applicability_profile_covers_four_categories() {
        let profile = applicability_profile();
        let cats = profile["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 4);
    }
}
