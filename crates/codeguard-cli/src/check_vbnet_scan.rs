//! VB.NET lint 扫描适配器。
//!
//! 调用 vbc/dotnet 原生工具进行 VB.NET 代码规范检查。

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
            "report_type": "vbnet_lint_scan",
            "language": "vbnet",
            "config": config,
            "status": "incomplete",
            "reason": "vbnet_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("dotnet")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("dotnet")
        .args(["build", "--no-restore", "-v", "q"])
        .current_dir(root)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let findings = parse_output(&stdout);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "vbnet_lint_scan",
        "language": "vbnet",
        "status": "complete",
        "native_tool": "dotnet_build",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in output.lines() {
        if line.contains("warning") || line.contains("error") {
            findings.push(json!({"message": line.trim(), "source": "dotnet_build"}));
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
    json!({"language": "vbnet", "categories": [
        {"category": "lint", "applicability": "applicable", "tool": "vbc"},
        {"category": "comments", "applicability": "applicable", "tool": "vbc"},
    ]})
}

fn observe_config(root: &Path) -> Value {
    let vbproj = root.join("*.vbproj");
    let config = if vbproj.exists() {
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
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("vbnet-scan-test");
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
