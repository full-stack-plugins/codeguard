//! Elixir lint 扫描适配器。
//!
//! 调用 Credo 原生工具进行 Elixir 代码规范检查。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "elixir_lint_scan",
        "language": "elixir",
        "config": config,
        "status": "incomplete",
        "reason": "elixir_native_tool_not_executed",
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
        "language": "elixir",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "credo"},
            {"category": "comments", "applicability": "applicable", "tool": "credo"},
        ],
    })
}

pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "elixir_rule_config",
        "config": config,
    })
}

pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

fn observe_config(root: &Path) -> Value {
    let credo_config = root.join(".credo.exs");
    let mix_exs = root.join("mix.exs");

    let config = if credo_config.exists() {
        "configured"
    } else if mix_exs.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if credo_config.exists() { ".credo.exs" } else if mix_exs.exists() { "mix.exs" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("elixir-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "elixir");
    }

    #[test]
    fn config_detection_identifies_credo_config() {
        let root = std::env::temp_dir().join("elixir-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".credo.exs"), "%{}").unwrap();
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
