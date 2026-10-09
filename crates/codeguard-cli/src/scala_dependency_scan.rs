//! Scala 依赖/CVE/安全/构建扫描适配器。
//!
//! 调用 sbt-dependency-check 原生工具进行 Scala 依赖检查和漏洞扫描。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "scala_dependency_scan",
        "language": "scala",
        "config": config,
        "status": "incomplete",
        "reason": "scala_sbt_not_executed",
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
        "language": "scala",
        "categories": [
            {"category": "dependencies", "applicability": "applicable", "tool": "sbt_dependency_check"},
            {"category": "cve", "applicability": "applicable", "tool": "sbt_dependency_check"},
            {"category": "security", "applicability": "applicable", "tool": "sbt_dependency_check"},
            {"category": "build", "applicability": "applicable", "tool": "sbt"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let build_sbt = root.join("build.sbt");
    let project_dir = root.join("project");

    let config = if project_dir.exists() {
        "configured"
    } else if build_sbt.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if project_dir.exists() { "project/" } else if build_sbt.exists() { "build.sbt" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("scala-dep-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
    }

    #[test]
    fn config_detection_identifies_project_dir() {
        let root = std::env::temp_dir().join("scala-dep-config-test");
        std::fs::create_dir_all(root.join("project")).unwrap();
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
