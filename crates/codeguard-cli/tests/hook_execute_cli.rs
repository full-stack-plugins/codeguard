#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-hook-execute-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn request(event: &str, paths: &[&str], write_outcome: &str) -> Value {
    json!({
        "schema_version":"1.0.0", "report_type":"hook_trigger_request",
        "input": {"event":event,"changed_paths":paths,"task_id":null,
            "write_outcome":write_outcome,"host_claims_blocking":false}
    })
}

fn run(project: &Project, request: &Value) -> (i32, Value) {
    run_with_extra(project, request, &[])
}

fn run_with_extra(project: &Project, request: &Value, extra: &[&str]) -> (i32, Value) {
    run_with_index(project, request, extra, None)
}

fn run_with_index(
    project: &Project,
    request: &Value,
    extra: &[&str],
    index: Option<&str>,
) -> (i32, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command
        .args([
            "hook",
            "execute",
            project.0.to_str().unwrap(),
            "--timeout=5s",
            "--format=json",
        ])
        .args(extra);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (
        result.status.code().unwrap(),
        serde_json::from_slice(&result.stdout).unwrap(),
    )
}

#[test]
#[ignore = "requires native Ruff 0.16.8 via CODEGUARD_RUFF_BIN"]
fn confirmed_python_edit_runs_native_ruff_only_for_selected_file() {
    let project = Project::new();
    fs::write(project.0.join("changed.py"), "import os\n").unwrap();
    fs::write(project.0.join("untouched.py"), "import sys\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, report) = run_with_extra(
        &project,
        &request("file_changed", &["changed.py"], "confirmed"),
        &["--ruff-tool", &tool],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["execution"], "local_observation");
    assert_eq!(
        report["local_feedback"]["files"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["local_feedback"]["files"][0]["path"], "changed.py");
    assert_eq!(
        report["local_feedback"]["files"][0]["run_status"],
        "findings"
    );
    assert_eq!(
        report["local_feedback"]["files"][0]["findings"][0]["rule_id"],
        "F401"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn confirmed_python_edit_routes_to_only_selected_native_feedback() {
    let project = Project::new();
    fs::write(project.0.join("changed.py"), "import os\n").unwrap();
    fs::write(project.0.join("untouched.py"), "import sys\n").unwrap();
    let (exit, report) = run(
        &project,
        &request("file_changed", &["changed.py"], "confirmed"),
    );
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "hook_execution_feedback");
    assert_eq!(report["plan"]["action"], "fast_file_check");
    assert_eq!(report["execution"], "local_observation");
    assert_eq!(report["local_feedback"]["scan_scope"], "selected_files");
    assert_eq!(
        report["local_feedback"]["requested_paths"],
        json!(["changed.py"])
    );
    assert_eq!(
        report["local_feedback"]["files"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["local_feedback"]["files"][0]["path"], "changed.py");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["host_blocking_verified"], false);
}

#[test]
fn session_start_only_discovers_without_running_a_checker() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let (exit, report) = run(&project, &request("session_start", &[], "unknown"));
    assert_eq!(exit, 3);
    assert_eq!(report["plan"]["action"], "discover_project");
    assert_eq!(report["execution"], "read_only_discovery");
    assert_eq!(
        report["local_feedback"]["report_type"],
        "hook_discovery_summary"
    );
    assert!(
        report["local_feedback"]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|language| language == "python")
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn failed_or_unknown_write_never_starts_source_check() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    for (outcome, action) in [("failed", "no_check"), ("unknown", "resolve_changed_scope")] {
        let (exit, report) = run(&project, &request("file_changed", &["app.py"], outcome));
        assert_eq!(exit, 3);
        assert_eq!(report["plan"]["action"], action);
        assert_eq!(report["execution"], "not_run");
        assert!(report["local_feedback"].is_null());
    }
}

#[test]
fn mixed_or_unimplemented_scope_is_visible_and_never_partially_scanned() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("App.java"), "class App {}\n").unwrap();
    let (exit, report) = run(
        &project,
        &request("file_changed", &["app.py", "App.java"], "confirmed"),
    );
    assert_eq!(exit, 3);
    assert_eq!(report["plan"]["action"], "fast_file_check");
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["reason"], "fast_scope_not_wired");
    assert!(report["local_feedback"].is_null());
}

#[test]
fn delivery_event_cannot_reuse_edit_feedback_or_claim_blocking() {
    let project = Project::new();
    let mut input = request("pre_commit", &["guess.py"], "unknown");
    input["input"]["host_claims_blocking"] = json!(true);
    let (exit, report) = run(&project, &input);
    assert_eq!(exit, 3);
    assert_eq!(report["plan"]["action"], "commit_gate");
    assert_eq!(report["plan"]["target_paths"], json!([]));
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["reason"], "delivery_gate_not_wired");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["host_blocking_verified"], false);
}

#[test]
fn pre_commit_observes_only_the_live_staged_index_without_claiming_delivery() {
    let project = Project::new();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    fs::write(project.0.join(".env"), "TOKEN=fixture\n").unwrap();
    fs::write(project.0.join("unstaged.pem"), "not staged\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "-f", ".env"])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    let git_tool = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .unwrap()
        .canonicalize()
        .unwrap();
    let mut input = request("pre_commit", &["unstaged.pem"], "unknown");
    input["input"]["host_claims_blocking"] = json!(true);
    let (exit, report) = run_with_extra(
        &project,
        &input,
        &["--git-tool", git_tool.to_str().unwrap()],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["execution"], "local_observation");
    assert_eq!(
        report["local_feedback"]["report_type"],
        "hook_git_index_summary"
    );
    assert_eq!(report["local_feedback"]["staged_entry_count"], 1);
    assert_eq!(report["local_feedback"]["violations"][0]["path"], ".env");
    assert_eq!(report["local_feedback"]["violation_count"], 1);
    assert_eq!(report["local_feedback"]["violations_truncated"], false);
    assert_eq!(report["local_feedback"]["index_observation"], "complete");
    assert_eq!(report["local_feedback"]["source_check"], "not_run");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["host_blocking_verified"], false);
    assert_eq!(report["soft_result_reused"], false);
}

#[test]
fn pre_commit_respects_alternate_index_and_missing_git_tool_is_incomplete() {
    let project = Project::new();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    fs::write(project.0.join("safe.py"), "pass\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "safe.py"])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    fs::write(project.0.join(".env"), "TOKEN=fixture\n").unwrap();
    let alternate = project.0.join("alternate.index");
    assert!(
        Command::new("git")
            .args(["add", "-f", ".env"])
            .env("GIT_INDEX_FILE", &alternate)
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    let git_tool = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .unwrap()
        .canonicalize()
        .unwrap();
    let input = request("pre_commit", &[], "unknown");
    let (exit, report) = run_with_index(
        &project,
        &input,
        &["--git-tool", git_tool.to_str().unwrap()],
        Some("alternate.index"),
    );
    assert_eq!(exit, 3);
    assert_eq!(report["execution"], "local_observation");
    assert_eq!(report["local_feedback"]["staged_entry_count"], 1);
    assert_eq!(report["local_feedback"]["violations"][0]["path"], ".env");

    let (exit, report) = run_with_extra(
        &project,
        &input,
        &["--git-tool", "/missing/codeguard-test-git"],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["reason"], "git_index_observation_failed");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn pre_commit_bounds_conversation_paths_without_hiding_total_violation_count() {
    let project = Project::new();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    for index in 0..33 {
        fs::write(project.0.join(format!(".env.{index}")), "fixture\n").unwrap();
    }
    assert!(
        Command::new("git")
            .args(["add", "-f", "."])
            .current_dir(&project.0)
            .status()
            .unwrap()
            .success()
    );
    let git_tool = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .unwrap()
        .canonicalize()
        .unwrap();
    let (exit, report) = run_with_extra(
        &project,
        &request("pre_commit", &[], "unknown"),
        &["--git-tool", git_tool.to_str().unwrap()],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["local_feedback"]["violation_count"], 33);
    assert_eq!(report["local_feedback"]["violations_truncated"], true);
    assert_eq!(
        report["local_feedback"]["violations"]
            .as_array()
            .unwrap()
            .len(),
        32
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn over_budget_edit_requests_new_scope_without_running_a_subset() {
    let project = Project::new();
    let paths: Vec<String> = (0..9).map(|index| format!("file{index}.py")).collect();
    let path_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let (exit, report) = run(&project, &request("file_changed", &path_refs, "confirmed"));
    assert_eq!(exit, 3);
    assert_eq!(report["plan"]["action"], "resolve_changed_scope");
    assert_eq!(
        report["plan"]["scope_resolution_reason"],
        "fast_scope_budget_exceeded"
    );
    assert_eq!(report["execution"], "not_run");
    assert!(report["local_feedback"].is_null());
}

#[test]
fn malformed_event_or_unbounded_timeout_is_rejected_without_feedback() {
    let project = Project::new();
    let mut invalid = request("file_changed", &["app.py"], "confirmed");
    invalid["input"]["changed_paths"] = json!(["../outside.py"]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "hook",
            "execute",
            project.0.to_str().unwrap(),
            "--timeout=5s",
            "--format=json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&invalid).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "hook",
            "execute",
            project.0.to_str().unwrap(),
            "--timeout=121s",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
