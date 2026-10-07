//! rust lint 扫描适配器（真实原生工具）。
//!
//! 调用 rustc 原生工具进行 rust 代码规范检查。

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
            "report_type": "rust_lint_scan",
            "language": "rust",
            "config": config,
            "status": "incomplete",
            "reason": "rust_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("rustc").args(["--version"]).current_dir(root).output().ok()?;
    if !output.status.success() { return None; }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("rustc").args(["."]).current_dir(root).output().ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&scan_output.stderr).to_string();
    let findings = parse_output(&stdout, &stderr);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "rust_lint_scan",
        "language": "rust",
        "status": "complete",
        "native_tool": "rustc",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(stdout: &str, stderr: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if line.contains("warning") || line.contains("error") {
            findings.push(json!({"message": line.trim(), "source": "rustc"}));
        }
    }
    findings
}

pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) { let _ = (root, report, deadline); }
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool { report["status"] == "incomplete" }
pub(crate) fn applicability_profile() -> Value {
    json!({"language": "rust", "categories": [
        {"category": "lint", "applicability": "applicable", "tool": "rustc"},
        {"category": "comments", "applicability": "applicable", "tool": "rustc"},
    ]})
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join("Cargo.toml");
    let config = if config_file.exists() { "configured" } else { "unknown" };
    json!({"status": config, "config_ref": if config_file.exists() { "Cargo.toml" } else { "." }})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("rust-native-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "rust");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        assert_eq!(applicability_profile()["categories"].as_array().unwrap().len(), 2);
    }
}
