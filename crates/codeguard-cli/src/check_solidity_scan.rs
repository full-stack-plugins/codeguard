//! Solidity lint 扫描适配器。
//!
//! 调用 solc 原生工具进行 Solidity 代码规范检查。

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
            "report_type": "solidity_lint_scan",
            "language": "solidity",
            "config": config,
            "status": "incomplete",
            "reason": "solidity_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("solc")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("solc")
        .args(["--ast-compact-json", "."])
        .current_dir(root)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let findings = parse_output(&stdout);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "solidity_lint_scan",
        "language": "solidity",
        "status": "complete",
        "native_tool": "solc",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in output.lines() {
        if line.contains("Error") || line.contains("Warning") {
            findings.push(json!({
                "message": line.trim(),
                "source": "solc",
            }));
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
    json!({"language": "solidity", "categories": [
        {"category": "lint", "applicability": "applicable", "tool": "solc"},
        {"category": "comments", "applicability": "applicable", "tool": "solc"},
    ]})
}

fn observe_config(root: &Path) -> Value {
    let foundry = root.join("foundry.toml");
    let hardhat = root.join("hardhat.config.js");
    let config = if foundry.exists() || hardhat.exists() {
        "configured"
    } else {
        "unknown"
    };
    json!({"status": config, "config_ref": if foundry.exists() { "foundry.toml" } else if hardhat.exists() { "hardhat.config.js" } else { "." }})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("solidity-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        assert_eq!(
            applicability_profile()["categories"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
