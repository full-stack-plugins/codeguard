#![cfg(unix)]
//! Python 独立注释入口；受控运输不证明原生语义或生产资格。
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new(config: bool) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-py-comments-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("app.py"),
            "import os\ndef f() -> int:\n    return 1\n",
        )
        .unwrap();
        if config {
            fs::write(
                root.join("ruff.toml"),
                "preview=true\n[lint]\nselect=['DOC201','F401']\n",
            )
            .unwrap();
        }
        let p = Self(root);
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", "--apply", "--format=json"])
            .arg(&p.0)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        p
    }
    fn tool(&self) -> PathBuf {
        let path = self.0.join("ruff");
        fs::write(&path, r#"#!/bin/sh
printf '%s\n' "$1" >> calls
if [ "$1" = '--version' ]; then echo 'ruff 0.16.8'; exit 0; fi
if [ "$2" = '--show-files' ]; then echo "$3"; exit 0; fi
if [ "$2" = '--show-settings' ]; then printf 'linter.rules.enabled = [\n\tmissing-return (DOC201),\n\tunused-import (F401),\n]\nlinter.per_file_ignores = {}\n'; exit 0; fi
if [ "$2" = '--no-cache' ]; then
 if [ "$3" = '--ignore-noqa' ]; then source=$6; else source=$5; fi
 if [ -f fixed ]; then printf '[]\n'; exit 0; fi
 printf '[{"code":"DOC201","message":"private doc text","filename":"%s","location":{"row":2,"column":1},"severity":"error"},{"code":"F401","message":"private import text","filename":"%s","location":{"row":1,"column":1},"severity":"error"}]\n' "$source" "$source"; exit 1
fi
exit 2
"#).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn comments(&self, extra: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "python", "--format=json"])
            .arg(&self.0)
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn doc_candidates_reuse_original_tasks_and_do_not_choose_convention_finding() {
    let p = Project::new(true);
    let tool = p.tool();
    let first = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(first["report_type"], "python_comments_feedback");
    assert_eq!(first["documentation_findings"].as_array().unwrap().len(), 1);
    assert_eq!(first["documentation_findings"][0]["rule_id"], "DOC201");
    assert_eq!(
        first["native_report"]["files"][0]["findings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let id = first["documentation_findings"][0]["finding_id"].clone();
    assert_eq!(first["next"]["repair_brief"]["task_id"], id);
    let repeat = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(repeat["documentation_findings"][0]["finding_id"], id);
    assert_eq!(repeat["detailed_contract_qualification"], "not_granted");
    assert!(!first.to_string().contains("private doc text"));
    assert!(!first.to_string().contains("private import text"));
    fs::write(p.0.join("fixed"), "").unwrap();
    let clean = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(clean["documentation_findings"], json!([]));
    assert_eq!(clean["next"]["repair_brief"]["task_id"], id);
}

#[test]
fn missing_configuration_is_a_preparation_task_and_never_executes_selected_tool() {
    let p = Project::new(false);
    let tool = p.tool();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["documentation_findings"], json!([]));
    assert_eq!(report["next"]["repair_brief"]["kind"], "blocker");
    assert!(!p.0.join("calls").exists());
    assert!(!p.0.join("ruff.toml").exists());
}

#[test]
fn uninitialized_comments_keeps_native_evidence_without_creating_workspace() {
    let p = Project::new(true);
    fs::remove_dir_all(p.0.join(".codeguard")).unwrap();
    let tool = p.tool();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(
        report["native_report"]["workspace_binding"],
        "uninitialized"
    );
    assert_eq!(
        report["documentation_findings"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["next"], Value::Null);
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn unknown_doc_rule_prefix_does_not_create_a_documentation_candidate() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("DOC201", "DOC999"),
    )
    .unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["documentation_findings"], json!([]));
    assert_ne!(report["next"]["repair_brief"]["native_rule_id"], "DOC999");
}

#[test]
fn malformed_arguments_stop_before_native_execution() {
    let p = Project::new(true);
    let tool = p.tool();
    for extra in [
        vec!["--format=json", "--format=human"],
        vec!["--timeout", "0s"],
        vec!["--ruff-tool", "relative"],
        vec!["--file", "../app.py"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "python"])
            .arg(&p.0)
            .args(["--ruff-tool", tool.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!p.0.join("calls").exists());
    }
}

#[test]
fn native_timeout_stays_incomplete_without_a_documentation_clean_claim() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("echo 'ruff 0.16.8'", "/bin/sleep 1; echo 'ruff 0.16.8'"),
    )
    .unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap(), "--timeout", "100ms"]);
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["documentation_rule_coverage"], "unverified");
    assert!(
        report["native_report"]["incomplete_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "request_deadline_exceeded"),
        "{report}"
    );
}

#[test]
fn nonexistent_root_keeps_structured_unavailable_native_report() {
    let p = Project::new(true);
    let tool = p.tool();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "python",
            "--format=json",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ])
        .arg(p.0.join("missing"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native_report"], Value::Null);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["reason"], "project_root_unavailable");
    assert!(!p.0.join("calls").exists());
}

#[test]
fn interrupted_native_probe_returns_cancelled_without_a_clean_claim() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("echo 'ruff 0.16.8'", "/bin/sleep 10; echo 'ruff 0.16.8'"),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "python",
            "--format=json",
            "--timeout",
            "5s",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ])
        .arg(&p.0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let wait_until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !p.0.join("calls").exists() && std::time::Instant::now() < wait_until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !p.0.join("calls").exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("原生版本探针未启动");
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["native_report"]["command_status"], "cancelled");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["documentation_rule_coverage"], "unverified");
}
