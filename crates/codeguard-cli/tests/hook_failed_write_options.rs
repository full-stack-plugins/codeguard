#![cfg(unix)]
//! 写入失败时忽略已校验的检查器配置，且不启动工具、不创建工作台。
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-failed-write-options-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("app.go"), "package main\n").unwrap();
        Self(path)
    }
    fn invoke(&self, options: &[(&str, &str)], outcome: &str) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args([
            "hook",
            "execute",
            self.0.to_str().unwrap(),
            "--timeout",
            "5s",
            "--format=json",
        ]);
        for (key, value) in options {
            command.args([key, value]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.go"],"task_id":null,"write_outcome":outcome,"host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
        child.wait_with_output().unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn every_registered_checker_configuration_is_inert_after_failed_write() {
    let p = Project::new();
    let marker = p.0.join("tool-ran");
    let tool = p.0.join("tool");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\n/usr/bin/touch '{}'\nexit 99\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let path = tool.to_str().unwrap();
    let mut options = Vec::new();
    for key in [
        "--ruff-tool",
        "--git-tool",
        "--cargo-tool",
        "--cargo-audit-tool",
        "--rustsec-db",
        "--pip-audit-tool",
        "--go-tool",
        "--zig-tool",
        "--erl-tool",
        "--swift-tool",
        "--kotlinc-tool",
        "--ruby-tool",
        "--shellcheck-tool",
        "--maven-tool",
        "--java-home",
        "--java-tool",
        "--checkstyle-jar",
        "--config",
        "--maven-repo",
        "--cve-data-dir",
        "--node-tool",
        "--npm-entry",
        "--userconfig",
        "--globalconfig",
        "--eslint-entry",
        "--cwd",
    ] {
        options.push((key, path));
    }
    for key in [
        "--pip-audit-version",
        "--repo-sha256",
        "--cve-data-sha256",
        "--npm-version",
        "--registry",
        "--eslint-version",
    ] {
        options.push((key, "configured"));
    }
    let out = p.invoke(&options, "failed");
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["reason"], "write_failed");
    assert!(report["local_feedback"].is_null());
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(!marker.exists());
    assert!(!p.0.join(".codeguard").exists());
}
#[test]
fn failed_write_still_rejects_ownership_and_malformed_arguments() {
    let p = Project::new();
    let oversized = "v".repeat(16 * 1024 + 1);
    for options in [
        vec![("--owner", "agent")],
        vec![("--lease-token", "token")],
        vec![("--go-tool", "relative")],
        vec![("--npm-version", "")],
        vec![("--npm-version", oversized.as_str())],
        vec![("--go-tool", "/missing"), ("--go-tool", "/other")],
        vec![("--invented-tool", "/missing")],
    ] {
        let out = p.invoke(&options, "failed");
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        assert!(out.stdout.is_empty());
        assert!(!p.0.join(".codeguard").exists());
    }
}
#[test]
fn confirmed_edit_reports_selected_go_failure_without_ignoring_it() {
    let p = Project::new();
    let out = p.invoke(&[("--go-tool", "/missing/go")], "confirmed");
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["local_feedback"]["go_syntax"]["tool_selection"]["source"],
        "explicit"
    );
    assert_eq!(
        report["local_feedback"]["go_syntax"]["files"][0]["native"]["status"],
        "incomplete"
    );
    assert!(!p.0.join(".codeguard").exists());
}
