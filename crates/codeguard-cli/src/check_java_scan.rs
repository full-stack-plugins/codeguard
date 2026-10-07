//! Java lint 扫描适配器。
//!
//! 调用 javac 原生工具进行 Java 代码规范检查。

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
            "report_type": "java_lint_scan",
            "language": "java",
            "config": config,
            "status": "incomplete",
            "reason": "java_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("javac").args(["-version"]).current_dir(root).output().ok()?;
    if !output.status.success() { return None; }
    let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
    // javac -Xlint 检查
    let scan_output = Command::new("javac")
        .args(["-Xlint:all", "-d", "/tmp/java-scan", "."])
        .current_dir(root)
        .output()
        .ok()?;
    let stderr = String::from_utf8_lossy(&scan_output.stderr).to_string();
    let findings = parse_output(&stderr);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "java_lint_scan",
        "language": "java",
        "status": "complete",
        "native_tool": "javac",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(output: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in output.lines() {
        if line.contains("warning") || line.contains("error") {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() >= 3 {
                findings.push(json!({
                    "path": parts[0],
                    "line": parts[1].parse::<u32>().unwrap_or(0),
                    "message": if parts.len() > 3 { parts[3] } else { line },
                    "source": "javac",
                }));
            }
        }
    }
    findings
}

pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) { let _ = (root, report, deadline); }
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool { report["status"] == "incomplete" }
pub(crate) fn applicability_profile() -> Value {
    json!({"language": "java", "categories": [
        {"category": "lint", "applicability": "applicable", "tool": "javac_xlint"},
        {"category": "comments", "applicability": "applicable", "tool": "javadoc"},
    ]})
}

fn observe_config(root: &Path) -> Value {
    let pom = root.join("pom.xml");
    let gradle = root.join("build.gradle");
    let gradle_kts = root.join("build.gradle.kts");
    let config = if pom.exists() || gradle.exists() || gradle_kts.exists() { "configured" } else { "unknown" };
    json!({"status": config, "config_ref": if pom.exists() { "pom.xml" } else if gradle.exists() { "build.gradle" } else if gradle_kts.exists() { "build.gradle.kts" } else { "." }})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("java-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "java");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn config_detection_identifies_pom() {
        let root = std::env::temp_dir().join("java-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("pom.xml"), "<project/>").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        assert_eq!(applicability_profile()["categories"].as_array().unwrap().len(), 2);
    }
}
