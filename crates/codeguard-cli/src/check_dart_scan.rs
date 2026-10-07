//! Dart lint 扫描适配器。
//!
//! 调用 dart analyze 原生工具进行 Dart 代码规范检查。

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
            "report_type": "dart_lint_scan",
            "language": "dart",
            "config": config,
            "status": "incomplete",
            "reason": "dart_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("dart")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    let scan_output = Command::new("dart")
        .args(["analyze", "--format=json"])
        .current_dir(root)
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let findings = parse_dart_json(&stdout);
    
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "dart_lint_scan",
        "language": "dart",
        "status": "complete",
        "native_tool": "dart_analyze",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_dart_json(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    
    if let Ok(json) = serde_json::from_str::<Value>(output) {
        if let Some(diagnostics) = json["diagnostics"].as_array() {
            for diag in diagnostics {
                findings.push(json!({
                    "path": diag["location"]["file"],
                    "line": diag["location"]["range"]["start"]["line"],
                    "column": diag["location"]["range"]["start"]["column"],
                    "severity": diag["severity"],
                    "message": diag["problemMessage"],
                    "source": diag["code"],
                }));
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
        "language": "dart",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "dart_analyze"},
            {"category": "comments", "applicability": "applicable", "tool": "dart_analyze"},
        ],
    })
}

fn observe_config(root: &Path) -> Value {
    let analysis_options = root.join("analysis_options.yaml");
    let pubspec = root.join("pubspec.yaml");
    let config = if analysis_options.exists() { "configured" } else if pubspec.exists() { "missing" } else { "unknown" };
    json!({
        "status": config,
        "config_ref": if analysis_options.exists() { "analysis_options.yaml" } else if pubspec.exists() { "pubspec.yaml" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("dart-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "dart");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn config_detection_identifies_analysis_options() {
        let root = std::env::temp_dir().join("dart-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("analysis_options.yaml"), "").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parse_dart_json_extracts_findings() {
        let json_str = r#"{"diagnostics": [{"location": {"file": "lib/main.dart", "range": {"start": {"line": 10, "column": 5}}}, "severity": "warning", "problemMessage": "unused variable", "code": "unused_local_variable"}]}"#;
        let findings = parse_dart_json(json_str);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["path"], "lib/main.dart");
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        assert_eq!(profile["categories"].as_array().unwrap().len(), 2);
    }
}
