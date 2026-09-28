#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-doctor-work-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn run(&self, command: &[&str], options: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(command)
            .arg(&self.0)
            .args(options)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
    fn init(&self) {
        assert_eq!(self.run(&["init"], &["--apply"]).0, 3);
    }
    fn tasks(&self) -> Vec<PathBuf> {
        fs::read_dir(self.0.join(".codeguard/tasks"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn doctor_failure_is_saved_and_synced_to_one_stable_environment_task() {
    let project = Project::new();
    project.init();
    let missing = project.0.join("missing-ruff");
    for _ in 0..2 {
        let (exit, report) = project.run(&["doctor"], &["--ruff-tool", missing.to_str().unwrap()]);
        assert_eq!(exit, 3);
        assert_eq!(report["persistence"], "synced_partial");
        assert_eq!(report["workspace_binding"], "bound");
        assert!(
            project
                .0
                .join(format!(
                    ".codeguard/reports/{}.json",
                    report["run_id"].as_str().unwrap()
                ))
                .exists()
        );
        assert_eq!(project.tasks().len(), 1);
    }
    let sync = project.run(&["work", "sync"], &[]).1;
    assert_eq!(sync["imported_reports"], 0);
    assert_eq!(sync["already_consumed_reports"], 2);
    assert_eq!(sync["failed_reports"], 0);
    let task = fs::read_to_string(&project.tasks()[0]).unwrap();
    assert!(task.contains("doctor"));
    assert!(task.contains("允许范围"));
    assert!(task.contains("关闭条件"));
    let next = project.run(&["next"], &[]).1;
    assert_eq!(next["repair_brief"]["checker_id"], "python.ruff.doctor");
    assert_eq!(next["repair_brief"]["recheck_argv"][1], "doctor");
    assert_eq!(next["delivery_decision"], "not_evaluated");
}

#[test]
fn no_selected_tool_saves_observation_without_inventing_missing_required_tool() {
    let project = Project::new();
    project.init();
    let report = project.run(&["doctor"], &[]).1;
    assert_eq!(report["persistence"], "synced_partial");
    assert_eq!(report["ruff_version"]["status"], "not_selected");
    assert!(project.tasks().is_empty());
    assert_eq!(report["backlog_sync"]["new_blockers"], 0);
}

#[test]
fn foreign_workspace_or_modified_consumed_report_is_rejected() {
    let project = Project::new();
    project.init();
    let report = project.run(&["doctor"], &[]).1;
    let path = project.0.join(format!(
        ".codeguard/reports/{}.json",
        report["run_id"].as_str().unwrap()
    ));
    let mut saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    saved["workspace_id"] = "other-workspace".into();
    fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert_eq!(project.run(&["work", "sync"], &[]).1["failed_reports"], 1);
    assert!(project.tasks().is_empty());
}

#[test]
fn uninitialized_doctor_does_not_initialize_or_persist() {
    let project = Project::new();
    let report = project.run(&["doctor"], &[]).1;
    assert_eq!(report["workspace_binding"], "uninitialized");
    assert_eq!(report["persistence"], "not_saved");
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn changing_diagnostic_reason_keeps_task_identity_and_each_observation() {
    let project = Project::new();
    project.init();
    let tool = project.0.join("ruff");
    project.run(&["doctor"], &["--ruff-tool", tool.to_str().unwrap()]);
    let tasks = project.tasks();
    fs::write(&tool, "not a native executable").unwrap();
    let report = project
        .run(&["doctor"], &["--ruff-tool", tool.to_str().unwrap()])
        .1;
    assert_eq!(
        report["ruff_version"]["reason"],
        "native_launcher_unrecognized"
    );
    assert_eq!(report["persistence"], "synced_partial");
    assert_eq!(project.tasks(), tasks);
    let id = tasks[0].file_stem().unwrap();
    let observations =
        fs::read_dir(project.0.join(".codeguard/state/observations").join(id)).unwrap();
    let mut reasons: Vec<String> = observations
        .map(|entry| {
            let value: Value =
                serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap();
            value["diagnostic_reason"].as_str().unwrap().into()
        })
        .collect();
    reasons.sort();
    assert_eq!(
        reasons,
        ["native_launcher_unrecognized", "tool_unavailable"]
    );
}

#[test]
fn fake_authority_and_unknown_fields_in_new_reports_are_rejected() {
    for (field, value) in [
        ("approved", Value::Bool(true)),
        ("readiness", "ready".into()),
    ] {
        let project = Project::new();
        project.init();
        let report = project.run(&["doctor"], &[]).1;
        let old = project.0.join(format!(
            ".codeguard/reports/{}.json",
            report["run_id"].as_str().unwrap()
        ));
        let mut saved: Value = serde_json::from_slice(&fs::read(old).unwrap()).unwrap();
        saved["run_id"] = "doctor-123-456-789".into();
        saved[field] = value;
        fs::write(
            project.0.join(".codeguard/reports/doctor-123-456-789.json"),
            serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
        assert_eq!(project.run(&["work", "sync"], &[]).1["failed_reports"], 1);
        assert!(project.tasks().is_empty());
    }
}

#[test]
fn report_directory_link_is_not_followed_and_failure_is_visible() {
    let project = Project::new();
    let outside = Project::new();
    project.init();
    fs::remove_dir(project.0.join(".codeguard/reports")).unwrap();
    std::os::unix::fs::symlink(&outside.0, project.0.join(".codeguard/reports")).unwrap();
    let report = project.run(&["doctor"], &[]).1;
    assert_eq!(report["persistence"], "backlog_update_failed");
    assert_eq!(report["ruff_version"]["status"], "not_selected");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
}

#[test]
fn real_version_recovery_does_not_close_task_or_grant_readiness() {
    let Ok(tool) = std::env::var("CODEGUARD_TEST_RUFF") else {
        return;
    };
    let project = Project::new();
    project.init();
    let missing = project.0.join("missing");
    project.run(&["doctor"], &["--ruff-tool", missing.to_str().unwrap()]);
    let tasks = project.tasks();
    let report = project.run(&["doctor"], &["--ruff-tool", &tool]).1;
    assert_eq!(report["ruff_version"]["status"], "observed_untrusted");
    assert_eq!(report["persistence"], "synced_partial");
    assert_eq!(report["readiness"], "unknown");
    assert_eq!(project.tasks(), tasks);
    assert_eq!(
        project.run(&["next"], &[]).1["repair_brief"]["checker_id"],
        "python.ruff.doctor"
    );
}

#[test]
fn task_verify_rechecks_doctor_failure_and_records_native_observation() {
    let project = Project::new();
    project.init();
    let missing = project.0.join("missing");
    let options = ["--ruff-tool", missing.to_str().unwrap()];
    project.run(&["doctor"], &options);
    let tasks = project.tasks();
    let id = tasks[0].file_stem().unwrap().to_str().unwrap();
    let (exit, report) = project.run(&["task", "verify", id], &options);
    assert_eq!(exit, 3);
    assert_eq!(report["observation"], "still_blocked");
    assert_eq!(report["event_persisted"], true);
    assert_eq!(report["reason"], Value::Null);
    assert_eq!(
        report["native_scan"]["ruff_version"]["reason"],
        "tool_unavailable"
    );
    assert_eq!(project.tasks(), tasks);
}

#[test]
fn doctor_task_verify_without_selection_does_not_invent_recovery() {
    let project = Project::new();
    project.init();
    let missing = project.0.join("missing");
    project.run(&["doctor"], &["--ruff-tool", missing.to_str().unwrap()]);
    let tasks = project.tasks();
    let id = tasks[0].file_stem().unwrap().to_str().unwrap();
    let report = project.run(&["task", "verify", id], &[]).1;
    assert_eq!(report["observation"], "incomplete");
    assert_eq!(report["event_persisted"], true);
    assert_eq!(
        report["native_scan"]["ruff_version"]["status"],
        "not_selected"
    );
    assert_eq!(project.tasks(), tasks);
}

#[test]
fn doctor_task_verify_rejects_script_before_execution() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new();
    project.init();
    let tool = project.0.join("ruff");
    project.run(&["doctor"], &["--ruff-tool", tool.to_str().unwrap()]);
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\ntouch '{}'\n",
            project.0.join("marker").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let tasks = project.tasks();
    let id = tasks[0].file_stem().unwrap().to_str().unwrap();
    let report = project
        .run(
            &["task", "verify", id],
            &["--ruff-tool", tool.to_str().unwrap()],
        )
        .1;
    assert_eq!(report["observation"], "still_blocked");
    assert_eq!(
        report["native_scan"]["ruff_version"]["reason"],
        "script_launcher_requires_isolation"
    );
    assert_eq!(report["event_persisted"], true);
    assert!(!project.0.join("marker").exists());
}

#[test]
fn task_verify_real_doctor_recovery_is_recorded_without_closure() {
    let Ok(tool) = std::env::var("CODEGUARD_TEST_RUFF") else {
        return;
    };
    let project = Project::new();
    project.init();
    let missing = project.0.join("missing");
    project.run(&["doctor"], &["--ruff-tool", missing.to_str().unwrap()]);
    let tasks = project.tasks();
    let id = tasks[0].file_stem().unwrap().to_str().unwrap();
    let report = project
        .run(&["task", "verify", id], &["--ruff-tool", &tool])
        .1;
    assert_eq!(
        report["observation"],
        "environment_restored_unverified_policy"
    );
    assert_eq!(report["event_persisted"], true);
    assert_eq!(report["native_scan"]["quality_checks"], "not_run");
    assert_eq!(report["reason"], Value::Null);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let next = project.run(&["next"], &[]).1;
    assert_eq!(
        next["repair_brief"]["verification_observation"], "environment_restored_unverified_policy",
        "{next}"
    );
    assert_eq!(next["repair_brief"]["disposition"], "verification_required");
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("仅版本观察")
    );
}
