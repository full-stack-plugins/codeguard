//! Go lint 扫描适配器。
//!
//! 调用 go vet 原生工具进行 Go 代码规范检查。

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
            "report_type": "go_lint_scan",
            "language": "go",
            "config": config,
            "status": "incomplete",
            "reason": "go_native_tool_not_available",
        }),
    }
}

fn try_native_tool(root: &Path) -> Option<Value> {
    let output = Command::new("go").args(["version"]).current_dir(root).output().ok()?;
    if !output.status.success() { return None; }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let scan_output = Command::new("go").args(["vet", "./..."]).current_dir(root).output().ok()?;
    let stdout = String::from_utf8_lossy(&scan_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&scan_output.stderr).to_string();
    let findings = parse_output(&stdout, &stderr);
    Some(json!({
        "schema_version": "0.1.0",
        "report_type": "go_lint_scan",
        "language": "go",
        "status": "complete",
        "native_tool": "go_vet",
        "native_version": version,
        "findings": findings,
    }))
}

fn parse_output(stdout: &str, stderr: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if line.contains(":") && (line.contains("warning") || line.contains("error") || line.contains("declared but not used")) {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() >= 3 {
                if let Ok(line_num) = parts[1].parse::<u32>() {
                    findings.push(json!({
                        "path": parts[0],
                        "line": line_num,
                        "message": if parts.len() > 3 { parts[3] } else { line },
                        "source": "go_vet",
                    }));
                }
            }
        }
    }
    findings
}

pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) { let _ = (root, report, deadline); }
pub(crate) fn prefers(report: &Value, _relative: &str) -> bool { report["status"] == "incomplete" }
pub(crate) fn applicability_profile() -> Value {
    json!({"language": "go", "categories": [
        {"category": "lint", "applicability": "applicable", "tool": "go_vet"},
        {"category": "comments", "applicability": "applicable", "tool": "go_vet"},
    ]})
}

fn observe_config(root: &Path) -> Value {
    let go_mod = root.join("go.mod");
    let config = if go_mod.exists() { "configured" } else { "unknown" };
    json!({"status": config, "config_ref": if go_mod.exists() { "go.mod" } else { "." }})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observe_reports_incomplete_or_complete() {
        let root = std::env::temp_dir().join("go-scan-test");
        std::fs::create_dir_all(&root).unwrap();
        let report = observe(&root, Instant::now());
        assert!(report["status"] == "incomplete" || report["status"] == "complete");
        assert_eq!(report["language"], "go");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn config_detection_identifies_go_mod() {
        let root = std::env::temp_dir().join("go-config-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("go.mod"), "module test").unwrap();
        let config = observe_config(&root);
        assert_eq!(config["status"], "configured");
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn applicability_profile_covers_lint_and_comments() {
        assert_eq!(applicability_profile()["categories"].as_array().unwrap().len(), 2);
    }
}
