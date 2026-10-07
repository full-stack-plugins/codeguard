//! Scala lint 扫描适配器。
//!
//! 调用 Scalafix 原生工具进行 Scala 代码规范检查。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "scala_lint_scan",
        "language": "scala",
        "config": config,
        "status": "incomplete",
        "reason": "scala_native_tool_not_executed",
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
            {"category": "lint", "applicability": "applicable", "tool": "scalafix"},
            {"category": "comments", "applicability": "applicable", "tool": "scalafix"},
        ],
    })
}

pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "scala_rule_config",
        "config": config,
    })
}

pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

fn observe_config(root: &Path) -> Value {
    let scalafix_config = root.join(".scalafix.conf");
    let build_sbt = root.join("build.sbt");

    let config = if scalafix_config.exists() {
        "configured"
    } else if build_sbt.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if scalafix_config.exists() { ".scalafix.conf" } else if build_sbt.exists() { "build.sbt" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("scala-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "scala");
    }

    #[test]
    fn config_detection_identifies_scalafix_config() {
        let root = std::env::temp_dir().join("scala-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".scalafix.conf"), "rules = []").unwrap();
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
