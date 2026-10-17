//! r lint 扫描适配器。
//!
//! 调用 lintr 原生工具进行 r 代码规范检查。

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
            "report_type": "r_lint_scan",
            "language": "r",
            "config": config,
            "status": "incomplete",
            "reason": "r_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("lintr")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("lintr")
        .args(["."])
        .current_dir(root)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let findings = parse_output(&stdout);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "r_lint_scan",
        "language": "r",
        "status": "complete",
        "native_tool": "lintr",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in output.lines() {
        if let Some(idx) = line.find(": ") {
            let location = &line[..idx];
            let message = &line[idx + 2..];
            let parts: Vec<&str> = location.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(line_num) = parts[1].parse::<u32>() {
                    findings.push(json!({
                        "path": parts[0],
                        "line": line_num,
                        "message": message,
                        "source": "lintr",
                    }));
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
        "language": "r",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "lintr"},
            {"category": "comments", "applicability": "applicable", "tool": "lintr"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join(".lintr");
    let fallback = root.join("DESCRIPTION");
    let config = if config_file.exists() {
        "configured"
    } else if fallback.exists() {
        "missing"
    } else {
        "unknown"
    };
    json!({
        "status": config,
        "config_ref": if config_file.exists() { ".lintr" } else if fallback.exists() { "DESCRIPTION" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("r-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "r");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        assert_eq!(profile["categories"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_output_extracts_findings() {
        let output = "file.lua:10: warning: unused variable";
        let findings = parse_output(output);
        assert_eq!(findings.len(), 1);
    }
}
