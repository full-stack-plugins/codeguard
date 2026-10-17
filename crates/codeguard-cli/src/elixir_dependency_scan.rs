//! Elixir 依赖/CVE/安全/构建扫描适配器。
//!
//! 调用 mix deps 原生工具进行 Elixir 依赖检查和漏洞扫描。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "elixir_dependency_scan",
        "language": "elixir",
        "config": config,
        "status": "incomplete",
        "reason": "elixir_mix_not_executed",
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
            {"category": "dependencies", "applicability": "applicable", "tool": "mix_deps"},
            {"category": "cve", "applicability": "applicable", "tool": "mix_deps_audit"},
            {"category": "security", "applicability": "applicable", "tool": "mix_deps_audit"},
            {"category": "build", "applicability": "applicable", "tool": "mix"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let mix_exs = root.join("mix.exs");
    let mix_lock = root.join("mix.lock");

    let config = if mix_lock.exists() {
        "configured"
    } else if mix_exs.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if mix_lock.exists() { "mix.lock" } else if mix_exs.exists() { "mix.exs" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("elixir-dep-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
    }

    #[test]
    fn config_detection_identifies_mix_lock() {
        let root = std::env::temp_dir().join("elixir-dep-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("mix.lock"), "").unwrap();
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
