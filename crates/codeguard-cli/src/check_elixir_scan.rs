//! Elixir lint 扫描适配器。
//!
//! 调用 Credo 原生工具进行 Elixir 代码规范检查。
//! 真实调用 mix credo 命令行工具，解析输出。

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
            "report_type": "elixir_lint_scan",
            "language": "elixir",
            "config": config,
            "status": "incomplete",
            "reason": "elixir_native_tool_not_available",
        }),
    }
}

/// 尝试调用真实 Credo 原生工具。
fn try_native_tool(root: &Path) -> Option<Value> {
    // 检查 mix 是否可用
    let output = Command::new("mix")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 调用 mix credo 进行扫描
    let scan_output = Command::new("mix")
        .args(["credo", "--strict", "--format=json"])
        .current_dir(root)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();

    // 解析 Credo JSON 输出
    let findings = parse_credo_json(&stdout);

    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "elixir_lint_scan",
        "language": "elixir",
        "status": "complete",
        "native_tool": "credo",
        "native_version": version,
        "findings": findings,
    }))
}

/// 解析 Credo JSON 输出。
fn parse_credo_json(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<Value>(output) {
        if let Some(issues) = json["issues"].as_array() {
            for issue in issues {
                findings.push(json!({
                    "path": issue["filename"],
                    "line": issue["line_no"],
                    "column": issue["column"],
                    "severity": issue["severity"],
                    "message": issue["message"],
                    "source": issue["check"],
                    "category": issue["category"],
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
        "language": "elixir",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "credo"},
            {"category": "comments", "applicability": "applicable", "tool": "credo"},
        ],
    })
}

pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "elixir_rule_config",
        "config": config,
    })
}

pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

fn observe_config(root: &Path) -> Value {
    let credo_config = root.join(".credo.exs");
    let mix_exs = root.join("mix.exs");

    let config = if credo_config.exists() {
        "configured"
    } else if mix_exs.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if credo_config.exists() { ".credo.exs" } else if mix_exs.exists() { "mix.exs" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("elixir-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        // mix 可用时返回 complete，不可用时返回 incomplete
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "elixir");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn config_detection_identifies_credo_config() {
        let root = std::env::temp_dir().join("elixir-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".credo.exs"), "%{}").unwrap();
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

    #[test]
    fn parse_credo_json_extracts_findings() {
        let json_str = r#"{"issues": [{"filename": "lib/main.ex", "line_no": 10, "column": 5, "severity": "warning", "message": "Module doc is missing", "check": "Credo.Check.Readability.ModuleDoc", "category": "readability"}]}"#;
        let findings = parse_credo_json(json_str);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["path"], "lib/main.ex");
        assert_eq!(findings[0]["line"], 10);
    }
}
