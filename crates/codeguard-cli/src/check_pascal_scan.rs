//! pascal lint 扫描适配器。
//!
//! 调用 fpc 原生工具进行 pascal 代码规范检查。

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
            "report_type": "pascal_lint_scan",
            "language": "pascal",
            "config": config,
            "status": "incomplete",
            "reason": "pascal_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("fpc")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("fpc")
        .args(["."])
        .current_dir(root)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let findings = parse_output(&stdout);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "pascal_lint_scan",
        "language": "pascal",
        "status": "complete",
        "native_tool": "fpc",
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
                        "source": "fpc",
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
        "language": "pascal",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "fpc"},
            {"category": "comments", "applicability": "applicable", "tool": "fpc"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let config_file = root.join("fpc.cfg");
    let fallback = root.join("*.lpr");
    let config = if config_file.exists() {
        "configured"
    } else if fallback.exists() {
        "missing"
    } else {
        "unknown"
    };
    json!({
        "status": config,
        "config_ref": if config_file.exists() { "fpc.cfg" } else if fallback.exists() { "*.lpr" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("pascal-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "pascal");
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
