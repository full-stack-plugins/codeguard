//! python 依赖/CVE/安全/构建扫描适配器。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "python_dependency_scan",
        "language": "python",
        "config": config,
        "status": "incomplete",
        "reason": "python_native_tool_not_executed",
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
        "language": "python",
        "categories": [
            {"category": "dependencies", "applicability": "applicable", "tool": "pip"},
            {"category": "cve", "applicability": "applicable", "tool": "pip"},
            {"category": "security", "applicability": "applicable", "tool": "pip"},
            {"category": "build", "applicability": "applicable", "tool": "pip"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join("config.toml");
    let config = if config_file.exists() {
        "configured"
    } else {
        "unknown"
    };
    json!({"status": config, "config_ref": "."})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete() {
        let root = std::env::temp_dir().join("python-dep-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_four_categories() {
        assert_eq!(
            applicability_profile()["categories"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }
}
