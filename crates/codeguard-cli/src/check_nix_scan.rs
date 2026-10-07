//! nix lint 扫描适配器。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "nix_lint_scan",
        "language": "nix",
        "config": config,
        "status": "incomplete",
        "reason": "nix_native_tool_not_executed",
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
        "language": "nix",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "nixpkgs-fmt"},
            {"category": "comments", "applicability": "applicable", "tool": "nixpkgs-fmt"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join(".editorconfig");
    let fallback = root.join("flake.nix");
    let config = if config_file.exists() { "configured" } else if fallback.exists() { "missing" } else { "unknown" };
    json!({
        "status": config,
        "config_ref": if config_file.exists() { ".editorconfig" } else if fallback.exists() { "flake.nix" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("nix-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        assert_eq!(profile["categories"].as_array().unwrap().len(), 2);
    }
}
