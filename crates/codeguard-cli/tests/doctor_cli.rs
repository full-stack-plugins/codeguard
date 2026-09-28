#![cfg(unix)]
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-doctor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("doctor")
            .arg(&self.0)
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn default_doctor_observes_configuration_without_selecting_or_running_tools() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let (exit, report) = project.run(&[]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "doctor_observation");
    assert_eq!(report["ruff_version"]["status"], "not_selected");
    assert_eq!(report["ruff_version"]["reason"], "ruff_tool_not_selected");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(
        report["profile_summary"]["checkers"][0]["configuration"],
        "missing"
    );
    assert!(!project.0.join("codeguard").exists());
    assert_eq!(
        fs::read_to_string(project.0.join("app.py")).unwrap(),
        "import os\n"
    );
}

#[test]
fn missing_tool_and_script_launcher_are_distinct_without_running_wrapper() {
    let project = Project::new();
    let missing = project.0.join("missing");
    let report = project.run(&["--ruff-tool", missing.to_str().unwrap()]).1;
    assert_eq!(report["ruff_version"]["reason"], "tool_unavailable");
    let wrapper = project.0.join("wrapper");
    fs::write(
        &wrapper,
        "#!/bin/sh\ntouch marker\nprintf 'ruff 0.16.8\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let report = project.run(&["--ruff-tool", wrapper.to_str().unwrap()]).1;
    assert_eq!(
        report["ruff_version"]["reason"],
        "script_launcher_requires_isolation"
    );
    assert!(!project.0.join("marker").exists());
    assert_eq!(report["ruff_version"]["status"], "incomplete");
}

#[test]
fn selected_real_ruff_runs_only_version_and_keeps_policy_unverified() {
    let Some(tool) = std::env::var_os("CODEGUARD_TEST_RUFF") else {
        return;
    };
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let (exit, report) = project.run(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(report["ruff_version"]["status"], "observed_untrusted");
    assert_eq!(report["ruff_version"]["version"], "ruff 0.16.8");
    assert_eq!(report["ruff_version"]["reason"], Value::Null);
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(report["quality_checks"], "not_run");
    assert!(report.get("findings").is_none());
    assert!(!report.to_string().contains(tool.to_str().unwrap()));
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn bad_arguments_do_not_start_diagnostics() {
    let project = Project::new();
    for args in [
        vec!["--install"],
        vec!["--timeout", "0s"],
        vec!["--ruff-tool", "relative"],
        vec!["--format=json", "--format=human"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("doctor")
            .arg(&project.0)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    assert!(!project.0.join("codeguard").exists());
}

#[test]
fn unreadable_project_and_unrecognized_launcher_have_specific_actions() {
    let project = Project::new();
    let path = project.0.join("not-a-project");
    fs::write(&path, "plain text").unwrap();
    let report = project.run(&["--ruff-tool", path.to_str().unwrap()]).1;
    assert_eq!(
        report["ruff_version"]["reason"],
        "native_launcher_unrecognized"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("doctor")
        .arg(&path)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ruff_version"]["reason"], "project_unavailable");
    assert!(report["profile_summary"].is_null());
}

#[test]
fn human_and_json_use_the_same_configuration_and_incomplete_semantics() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let report = project.run(&[]).1;
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("doctor")
        .arg(&project.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("准备 unknown"));
    assert!(text.contains("交付 not_evaluated"));
    assert!(text.contains(report["ruff_version"]["reason"].as_str().unwrap()));
    for checker in report["profile_summary"]["checkers"].as_array().unwrap() {
        assert!(text.contains(checker["checker_id"].as_str().unwrap()));
        assert!(text.contains(checker["configuration"].as_str().unwrap()));
    }
}
