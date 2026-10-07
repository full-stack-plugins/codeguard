//! lua lint 扫描适配器。
//!
//! 调用 luacheck 原生工具进行 lua 代码规范检查。

use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "lua_lint_scan",
        "language": "lua",
        "config": config,
        "status": "incomplete",
        "reason": "lua_native_tool_not_executed",
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
        "language": "lua",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "luacheck"},
            {"category": "comments", "applicability": "applicable", "tool": "luacheck"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join(".luacheckrc");
    let fallback = root.join("config.lua");

    let config = if config_file.exists() {
        "configured"
    } else if fallback.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if config_file.exists() { ".luacheckrc" } else if fallback.exists() { "config.lua" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("lua-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "lua");
    }

    #[test]
    fn config_detection_identifies_config() {
        let root = std::env::temp_dir().join("lua-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".luacheckrc"), "").unwrap();
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
