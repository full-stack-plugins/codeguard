#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cg-sarif-{}-{nonce}-{}",
            std::process::id(),
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='cg-sarif'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n").unwrap();
        Self(root)
    }

    fn cargo_tool(&self) -> PathBuf {
        let tool = self.0.join("cargo-native");
        fs::write(&tool, "#!/bin/sh\nprintf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"message\":\"token=private\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_all_sarif_keeps_native_finding_and_incomplete_state_without_raw_message() {
    let project = Project::new();
    let tool = project.cargo_tool();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format",
            "sarif",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "incomplete"
    );
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("token=private")
    );
}

#[test]
fn check_java_sarif_with_no_native_finding_does_not_claim_success() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=sarif",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["runs"][0]["results"], serde_json::json!([]));
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "not_evaluated"
    );
}

#[test]
fn unwritable_report_destination_preserves_native_finding_on_stdout() {
    let project = Project::new();
    let tool = project.cargo_tool();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format",
            "sarif",
            "--output",
            project
                .0
                .join("missing-parent/report.sarif")
                .to_str()
                .unwrap(),
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardExportStatus"],
        "failed"
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardExportReason"],
        "output_parent_unavailable"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("报告导出失败"));
}

#[test]
fn sarif_output_file_matches_stdout_after_atomic_export() {
    let project = Project::new();
    let target = project.0.join("report.sarif");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=sarif",
            "--output",
            target.to_str().unwrap(),
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let from_stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    let from_file: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(from_file, from_stdout);
    assert_eq!(
        from_file["runs"][0]["properties"]["codeguardExportStatus"],
        "saved"
    );
}

#[test]
fn output_cannot_replace_existing_source_file() {
    let project = Project::new();
    let source = project.0.join("src/lib.rs");
    let before = fs::read(&source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=sarif",
            "--output",
            source.to_str().unwrap(),
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(fs::read(&source).unwrap(), before);
    assert!(serde_json::from_slice::<Value>(&output.stdout).is_ok());
    assert!(String::from_utf8_lossy(&output.stderr).contains("报告导出失败"));
}

#[test]
fn json_output_can_atomically_replace_prior_codeguard_report() {
    let project = Project::new();
    let target = project.0.join("report.json");
    fs::write(&target, b"{\"report_type\":\"check_feedback\"}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "java",
            project.0.to_str().unwrap(),
            "--format=json",
            "--output",
            target.to_str().unwrap(),
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let from_stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    let from_file: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(from_file, from_stdout);
    assert_eq!(from_file["report_type"], "check_feedback");
    assert_eq!(from_file["export"]["status"], "saved");
}
