//! PHP lint 扫描适配器。
//!
//! 调用 PHP_CodeSniffer 原生工具进行 PHP 代码规范检查。
//! 真实调用 phpcs 命令行工具，解析 JSON 输出。

use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// 观察 PHP lint 扫描结果。
pub(crate) fn observe(root: &Path, deadline: Instant) -> Value {
    let _ = deadline;
    let config = observe_config(root);

    // 尝试调用真实 PHP_CodeSniffer
    let tool_result = try_native_tool(root);

    match tool_result {
        Some(result) => result,
        None => json!({
            "schema_version": "0.1.0",
            "report_type": "php_lint_scan",
            "language": "php",
            "config": config,
            "status": "incomplete",
            "reason": "php_native_tool_not_available",
        }),
    }
}

/// 尝试调用真实 PHP_CodeSniffer 原生工具。
fn try_native_tool(root: &Path) -> Option<Value> {
    // 检查 phpcs 是否可用
    let output = Command::new("phpcs")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 调用 phpcs 进行扫描
    let scan_output = Command::new("phpcs")
        .args([
            "--standard=PSR12",
            "--report=json",
            "--report-width=120",
            ".",
        ])
        .current_dir(root)
        .output()
        .ok()?;

    if !scan_output.status.success() {
        // phpcs 返回非零表示有违规
        let stdout = String::from_utf8_lossy(&scan_output.stdout);
        if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
            return Some(parse_phpcs_report(&json, &version));
        }
    }

    // 无违规
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "php_lint_scan",
        "language": "php",
        "status": "complete",
        "native_tool": "phpcs",
        "native_version": version,
        "findings": [],
    }))
}

/// 解析 PHP_CodeSniffer JSON 报告。
fn parse_phpcs_report(report: &Value, version: &str) -> Value {
    let mut findings = Vec::new();

    if let Some(files) = report["files"].as_object() {
        for (path, file_data) in files {
            if let Some(messages) = file_data["messages"].as_array() {
                for msg in messages {
                    findings.push(json!({
                        "path": path,
                        "line": msg["line"],
                        "column": msg["column"],
                        "severity": msg["severity"],
                        "message": msg["message"],
                        "source": msg["source"],
                        "fixable": msg["fixable"],
                    }));
                }
            }
        }
    }

    json!({
        "schema_version": "0.1.0",
        "report_type": "php_lint_scan",
        "language": "php",
        "status": "complete",
        "native_tool": "phpcs",
        "native_version": version,
        "findings": findings,
    })
}

/// 刷新报告。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let _ = (root, report, deadline);
}

/// 检查是否优先使用原生工具。
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool {
    report["status"] == "incomplete"
}

/// 适用性档案。
pub(crate) fn applicability_profile() -> Value {
    json!({
        "language": "php",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "phpcs"},
            {"category": "comments", "applicability": "applicable", "tool": "phpcs"},
        ],
    })
}

/// 观察规则配置状态。
pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "php_rule_config",
        "config": config,
    })
}

/// 复检规则配置。
pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

/// 观察 PHP 配置状态。
fn observe_config(root: &Path) -> Value {
    let phpcs_config = root.join(".phpcs.xml");
    let phpcs_config_alt = root.join("phpcs.xml");
    let composer = root.join("composer.json");

    let config = if phpcs_config.exists() || phpcs_config_alt.exists() {
        "configured"
    } else if composer.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if phpcs_config.exists() { ".phpcs.xml" } else if phpcs_config_alt.exists() { "phpcs.xml" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("php-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        // phpcs 不可用时返回 incomplete
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "php");
    }

    #[test]
    fn config_detection_identifies_phpcs_config() {
        let root = std::env::temp_dir().join("php-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("phpcs.xml"), "<ruleset/>").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        let profile = applicability_profile();
        let cats = profile["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0]["category"], "lint");
        assert_eq!(cats[1]["category"], "comments");
    }

    #[test]
    fn parse_phpcs_report_extracts_findings() {
        let report = json!({
            "files": {
                "test.php": {
                    "messages": [
                        {"line": 1, "column": 1, "severity": 5, "message": "error", "source": "PSR12", "fixable": true}
                    ]
                }
            }
        });
        let result = parse_phpcs_report(&report, "3.0.0");
        assert_eq!(result["status"], "complete");
        assert_eq!(result["findings"].as_array().unwrap().len(), 1);
    }
}
