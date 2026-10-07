//! Lua lint 扫描适配器。
//!
//! 调用 luacheck 原生工具进行 Lua 代码规范检查。
//! 真实调用 luacheck 命令行工具，解析输出。

use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);
    let tool_result = try_native_tool(root);

    match tool_result {
        Some(result) => result,
        None => json!({
            "schema_version": "0.1.0",
            "report_type": "lua_lint_scan",
            "language": "lua",
            "config": config,
            "status": "incomplete",
            "reason": "lua_native_tool_not_available",
        }),
    }
}

/// 尝试调用真实 luacheck 原生工具。
fn try_native_tool(root: &Path) -> Option<Value> {
    // 检查 luacheck 是否可用
    let output = Command::new("luacheck")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 调用 luacheck 进行扫描
    let scan_output = Command::new("luacheck")
        .args(["--formatter=json", "--no-color", "."])
        .current_dir(root)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();

    // 解析 luacheck JSON 输出
    let findings = parse_luacheck_json(&stdout);

    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "lua_lint_scan",
        "language": "lua",
        "status": "complete",
        "native_tool": "luacheck",
        "native_version": version,
        "findings": findings,
    }))
}

/// 解析 luacheck JSON 输出。
fn parse_luacheck_json(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<Value>(output) {
        if let Some(warnings) = json.as_array() {
            for file_warnings in warnings {
                if let Some(warnings_array) = file_warnings["warnings"].as_array() {
                    for warning in warnings_array {
                        findings.push(json!({
                            "path": file_warnings["filename"],
                            "line": warning["line"],
                            "column": warning["column"],
                            "severity": warning["severity"],
                            "message": warning["message"],
                            "source": warning["code"],
                            "category": warning["category"],
                        }));
                    }
                }
            }
        }
    }

    findings
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
    let luacheckrc = root.join(".luacheckrc");
    let config = if luacheckrc.exists() {
        "configured"
    } else {
        "unknown"
    };
    json!({
        "status": config,
        "config_ref": if luacheckrc.exists() { ".luacheckrc" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("lua-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "lua");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn config_detection_identifies_luacheckrc() {
        let root = std::env::temp_dir().join("lua-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".luacheckrc"), "-- config").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        assert_eq!(profile["categories"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_luacheck_json_extracts_findings() {
        let json_str = r#"[{"filename": "main.lua", "warnings": [{"line": 10, "column": 5, "severity": "warning", "message": "unused variable", "code": "W211", "category": "unused"}]}]"#;
        let findings = parse_luacheck_json(json_str);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["path"], "main.lua");
    }
}
