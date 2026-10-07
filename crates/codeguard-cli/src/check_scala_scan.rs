//! Scala lint 扫描适配器。
//!
//! 调用 Scalafix 原生工具进行 Scala 代码规范检查。
//! 真实调用 scalafix/sbt 命令行工具，解析输出。

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
            "report_type": "scala_lint_scan",
            "language": "scala",
            "config": config,
            "status": "incomplete",
            "reason": "scala_native_tool_not_available",
        }),
    }
}

/// 尝试调用真实 Scalafix 原生工具。
fn try_native_tool(root: &Path) -> Option<Value> {
    // 检查 sbt 是否可用
    let output = Command::new("sbt")
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // 调用 sbt scalafix 进行扫描
    let scan_output = Command::new("sbt")
        .args([
            "scalafix",
            "--check",
            "--rules=DisableSyntax,ExplicitResultTypes",
        ])
        .current_dir(root)
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&scan_output.stderr).to_string();
    
    // 解析 Scalafix 输出
    let findings = parse_scalafix_output(&stdout, &stderr);
    
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "scala_lint_scan",
        "language": "scala",
        "status": "complete",
        "native_tool": "scalafix",
        "native_version": version,
        "findings": findings,
    }))
}

/// 解析 Scalafix 输出。
fn parse_scalafix_output(stdout: &str, stderr: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    
    // Scalafix 输出格式：文件:行:列: 规则: 消息
    for line in stdout.lines().chain(stderr.lines()) {
        if let Some(idx) = line.find(": ") {
            let location = &line[..idx];
            let message = &line[idx + 2..];
            
            // 解析 文件:行:列
            let parts: Vec<&str> = location.split(':').collect();
            if parts.len() >= 3 {
                if let (Ok(line_num), Ok(col_num)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                    findings.push(json!({
                        "path": parts[0],
                        "line": line_num,
                        "column": col_num,
                        "message": message,
                        "source": "scalafix",
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
        "language": "scala",
        "categories": [
            {"category": "lint", "applicability": "applicable", "tool": "scalafix"},
            {"category": "comments", "applicability": "applicable", "tool": "scalafix"},
        ],
    })
}

pub(crate) fn observe_rule_config(root: &Path, _deadline: Instant) -> Value {
    let config = observe_config(root);
    json!({
        "schema_version": "0.1.0",
        "report_type": "scala_rule_config",
        "config": config,
    })
}

pub(crate) fn rule_config_recheck(_root: &Path, _report: &Value) -> Option<&'static str> {
    None
}

fn observe_config(root: &Path) -> Value {
    let scalafix_config = root.join(".scalafix.conf");
    let build_sbt = root.join("build.sbt");

    let config = if scalafix_config.exists() {
        "configured"
    } else if build_sbt.exists() {
        "missing"
    } else {
        "unknown"
    };

    json!({
        "status": config,
        "config_ref": if scalafix_config.exists() { ".scalafix.conf" } else if build_sbt.exists() { "build.sbt" } else { "." },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_reports_incomplete_without_native_tool() {
        let root = std::env::temp_dir().join("scala-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["language"], "scala");
    }

    #[test]
    fn config_detection_identifies_scalafix_config() {
        let root = std::env::temp_dir().join("scala-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".scalafix.conf"), "rules = []").unwrap();
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
    fn parse_scalafix_output_extracts_findings() {
        let stdout = "src/main/scala/Main.scala:10:5: DisableSyntax: Use of 'return' is disabled";
        let findings = parse_scalafix_output(stdout, "");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["path"], "src/main/scala/Main.scala");
        assert_eq!(findings[0]["line"], 10);
        assert_eq!(findings[0]["column"], 5);
    }
}
