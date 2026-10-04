#![cfg(unix)]

use codeguard_runtime::TaskFileLock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn lease_schemas_keep_quality_decision_unverified() {
    for source in [
        include_str!("../../../schemas/task-lease-state.schema.json"),
        include_str!("../../../schemas/task-lease-response.schema.json"),
    ] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert!(!source.contains("\"allow\""));
    }
}

#[test]
fn attempt_schemas_keep_local_attempts_separate_from_quality_resolution() {
    for source in [
        include_str!("../../../schemas/task-attempt-event.schema.json"),
        include_str!("../../../schemas/task-attempt-response.schema.json"),
    ] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert!(schema["$id"].as_str().unwrap().contains("task-attempt"));
        assert!(!source.contains("resolved_by_code_fix"));
        assert!(!source.contains("\"allow\""));
    }
}

struct Project {
    root: PathBuf,
    task_id: String,
}

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-lease-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), "import os\n").unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        let workspace: Value =
            serde_json::from_slice(&fs::read(root.join(".codeguard/workspace.json")).unwrap())
                .unwrap();
        let fingerprint = "a".repeat(64);
        let task_id = format!("CG-{}", &fingerprint[..32]);
        let source_sha256 = format!(
            "{:x}",
            Sha256::digest(fs::read(root.join("app.py")).unwrap())
        );
        let report = json!({
            "schema_version":"0.4.0", "report_type":"python_lint_feedback",
            "operation":"lint", "language":"python", "run_id":"lint-1-100",
            "workspace_binding":"bound", "workspace_id":workspace["workspace_id"],
            "command_status":"incomplete", "delivery_decision":"not_evaluated",
            "tool_approval":"unverified",
            "files":[{"path":"app.py", "source_sha256":source_sha256,
                "run_status":"findings", "recheck_cwd":root,
                "recheck_argv":["ruff","check","app.py"],
                "findings":[{"finding_id":task_id,"finding_fingerprint":fingerprint,
                    "path":"app.py","rule_id":"F401","line":1}]}]
        });
        fs::write(
            root.join(".codeguard/reports/lint-1-100.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["work", "sync", root.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap();
        assert_eq!(sync.status.code(), Some(3));
        let result: Value = serde_json::from_slice(&sync.stdout).unwrap();
        assert_eq!(result["new_findings"], 1);
        Self { root, task_id }
    }

    fn run(&self, operation: &str, owner: &str, token: Option<&str>) -> (i32, Value) {
        let mut args = vec![
            "task",
            operation,
            self.task_id.as_str(),
            self.root.to_str().unwrap(),
            "--owner",
            owner,
        ];
        if let Some(token) = token {
            args.extend(["--lease-token", token]);
        }
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn attempt(&self, operation: &str, owner: &str, token: &str, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "attempt",
                operation,
                self.task_id.as_str(),
                self.root.to_str().unwrap(),
                "--owner",
                owner,
                "--lease-token",
                token,
            ])
            .args(extra)
            .arg("--format=json")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn next(&self) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", self.root.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn token_and_generation_prevent_same_owner_from_reusing_an_old_lease() {
    let project = Project::new();
    let (exit, first) = project.run("claim", "agent-a", None);
    assert_eq!(exit, 0);
    assert_eq!(first["operation"], "task_claim");
    assert_eq!(first["generation"], 1);
    assert_eq!(first["delivery_decision"], "not_evaluated");
    let token = first["lease_token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    assert_eq!(project.run("claim", "agent-a", None).0, 3);
    assert_eq!(
        project.run("heartbeat", "agent-a", Some(&"0".repeat(64))).0,
        3
    );
    assert_eq!(project.run("heartbeat", "agent-a", Some(token)).0, 0);
    assert_eq!(project.run("release", "agent-b", Some(token)).0, 3);
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 0);
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 0);
    let (exit, second) = project.run("claim", "agent-a", None);
    assert_eq!(exit, 0);
    assert_eq!(second["generation"], 2);
    assert_ne!(second["lease_token"], first["lease_token"]);
    assert_eq!(project.run("heartbeat", "agent-a", Some(token)).0, 3);
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 3);
}

#[test]
fn attempt_requires_valid_lease_and_records_no_progress_without_closing_the_task() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let wrong = "0".repeat(64);
    let (exit, denied) = project.attempt("start", "agent-a", &wrong, &["--action-id", &action]);
    assert_eq!(exit, 3);
    assert_eq!(denied["reason"], "lease_token_mismatch");
    let (exit, started) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
    assert_eq!(exit, 0);
    let attempt_id = started["attempt_id"].as_str().unwrap();
    let (second_exit, second_start) =
        project.attempt("start", "agent-a", token, &["--action-id", &action]);
    assert_eq!(second_exit, 3);
    assert_eq!(second_start["reason"], "attempt_already_open");
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            project.task_id.as_str(),
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let blocked: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(blocked["reason"], "attempt_still_open");
    assert!(blocked["native_scan"].is_null());
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 3);
    let (exit, finished) = project.attempt(
        "finish",
        "agent-a",
        token,
        &[
            "--attempt-id",
            attempt_id,
            "--outcome",
            "no-change",
            "--note-code",
            "no_change",
        ],
    );
    assert_eq!(exit, 0);
    assert_eq!(finished["outcome"], "no-change");
    assert_eq!(finished["observed_change"], false);
    assert_eq!(
        project
            .attempt(
                "finish",
                "agent-a",
                token,
                &[
                    "--attempt-id",
                    attempt_id,
                    "--outcome",
                    "no-change",
                    "--note-code",
                    "no_change"
                ]
            )
            .0,
        0
    );
    assert_eq!(
        project
            .attempt(
                "finish",
                "agent-a",
                token,
                &[
                    "--attempt-id",
                    attempt_id,
                    "--outcome",
                    "failed",
                    "--note-code",
                    "execution_failed"
                ]
            )
            .0,
        3
    );
    assert_eq!(
        project.next()["repair_brief"]["history"]["no_progress_count"],
        1
    );
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 0);
}

#[test]
fn repeated_no_progress_stops_recommending_the_same_action() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    for _ in 0..2 {
        let (_, started) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
        let attempt_id = started["attempt_id"].as_str().unwrap();
        assert_eq!(
            project
                .attempt(
                    "finish",
                    "agent-a",
                    token,
                    &[
                        "--attempt-id",
                        attempt_id,
                        "--outcome",
                        "no-change",
                        "--note-code",
                        "no_change"
                    ]
                )
                .0,
            0
        );
    }
    let next = project.next();
    assert_eq!(next["disposition"], "needs_decision");
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 2);
    assert_eq!(next["repair_brief"]["disposition"], "needs_decision");
    assert_eq!(
        project
            .attempt("start", "agent-a", token, &["--action-id", &action])
            .0,
        3
    );
}

#[test]
fn expired_claim_records_abandoned_before_new_generation_and_old_finish_cannot_change_it() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, first) = project.run("claim", "agent-a", None);
    let old_token = first["lease_token"].as_str().unwrap();
    let (_, started) = project.attempt("start", "agent-a", old_token, &["--action-id", &action]);
    let attempt_id = started["attempt_id"].as_str().unwrap();
    let lease_path = project
        .root
        .join(format!(".codeguard/state/leases/{}.json", project.task_id));
    let mut lease: Value = serde_json::from_slice(&fs::read(&lease_path).unwrap()).unwrap();
    lease["expires_at"] = json!(1);
    fs::write(&lease_path, serde_json::to_vec_pretty(&lease).unwrap()).unwrap();
    let (exit, second) = project.run("claim", "agent-b", None);
    assert_eq!(exit, 0);
    assert_eq!(second["generation"], 2);
    let finish_path = project.root.join(format!(
        ".codeguard/findings/{}/events/attempt-{}-finish.json",
        project.task_id, attempt_id
    ));
    let finish: Value = serde_json::from_slice(&fs::read(&finish_path).unwrap()).unwrap();
    assert_eq!(finish["outcome"], "abandoned");
    assert_eq!(
        project.next()["repair_brief"]["history"]["no_progress_count"],
        1
    );
    assert_eq!(
        project
            .attempt(
                "finish",
                "agent-a",
                old_token,
                &[
                    "--attempt-id",
                    attempt_id,
                    "--outcome",
                    "failed",
                    "--note-code",
                    "execution_failed"
                ]
            )
            .0,
        3
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&finish_path).unwrap()).unwrap(),
        finish
    );
}

#[test]
fn finish_replay_is_idempotent_after_lease_released_and_cannot_hide_a_real_edit() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let (_, started) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
    let attempt_id = started["attempt_id"].as_str().unwrap();
    fs::write(project.root.join("app.py"), "import sys\n").unwrap();
    let (exit, denied) = project.attempt(
        "finish",
        "agent-a",
        token,
        &[
            "--attempt-id",
            attempt_id,
            "--outcome",
            "no-change",
            "--note-code",
            "no_change",
        ],
    );
    assert_eq!(exit, 3);
    assert_eq!(denied["reason"], "outcome_observation_conflict");
    let args = [
        "--attempt-id",
        attempt_id,
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "source_edit",
    ];
    let (exit, finished) = project.attempt("finish", "agent-a", token, &args);
    assert_eq!(exit, 0);
    assert_eq!(finished["observed_change"], true);
    assert_eq!(project.run("release", "agent-a", Some(token)).0, 0);
    let (exit, replay) = project.attempt("finish", "agent-a", token, &args);
    assert_eq!(exit, 0);
    assert_eq!(replay, finished);
}

#[test]
fn changing_action_name_cannot_reset_the_attempt_budget() {
    let project = Project::new();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let (exit, report) = project.attempt(
        "start",
        "agent-a",
        token,
        &["--action-id", "restore-checker-environment"],
    );
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "action_id_invalid");
    assert_eq!(
        project.next()["repair_brief"]["history"]["attempt_count"],
        0
    );
}

#[test]
fn a_changed_source_must_be_rechecked_before_starting_another_repair_attempt() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::write(
        project.root.join("app.py"),
        "import os\n# changed outside attempt\n",
    )
    .unwrap();
    assert_eq!(project.next()["disposition"], "verification_required");
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let (exit, denied) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
    assert_eq!(exit, 3);
    assert_eq!(denied["reason"], "task_not_actionable");
}

#[test]
fn ready_to_verify_requires_original_recheck_and_failed_rechecks_consume_budget() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let prior = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            project.task_id.as_str(),
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(prior.status.code(), Some(3));
    let prior_report: Value = serde_json::from_slice(&prior.stdout).unwrap();
    assert_eq!(prior_report["event_persisted"], true);
    for _ in 0..2 {
        let (start_exit, started) =
            project.attempt("start", "agent-a", token, &["--action-id", &action]);
        assert_eq!(start_exit, 0);
        let attempt_id = started["attempt_id"].as_str().unwrap();
        assert_eq!(
            project
                .attempt(
                    "finish",
                    "agent-a",
                    token,
                    &[
                        "--attempt-id",
                        attempt_id,
                        "--outcome",
                        "ready-to-verify",
                        "--note-code",
                        "source_edit"
                    ]
                )
                .0,
            0
        );
        let (premature_exit, premature) =
            project.attempt("start", "agent-a", token, &["--action-id", &action]);
        assert_eq!(premature_exit, 3);
        assert_eq!(premature["reason"], "verification_required_before_retry");
        let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                project.task_id.as_str(),
                project.root.to_str().unwrap(),
                "--owner",
                "agent-a",
                "--lease-token",
                token,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(verify.status.code(), Some(3));
        let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
        assert_eq!(observation["event_persisted"], true);
        assert_eq!(observation["observation"], "incomplete");
        let event = project.root.join(format!(
            ".codeguard/findings/{}/events/verify-{}.json",
            project.task_id,
            observation["native_scan"]["run_id"].as_str().unwrap()
        ));
        let recorded: Value = serde_json::from_slice(&fs::read(event).unwrap()).unwrap();
        assert_eq!(recorded["attempt_id"], attempt_id);
    }
    let (next_exit, next_start) =
        project.attempt("start", "agent-a", token, &["--action-id", &action]);
    assert_eq!(next_exit, 3);
    assert_eq!(next_start["reason"], "no_progress_budget_exhausted");
}

#[test]
fn missing_private_recheck_report_requires_a_new_native_recheck() {
    let project = Project::new();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = project.run("claim", "agent-a", None);
    let token = lease["lease_token"].as_str().unwrap();
    let (_, started) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
    let attempt_id = started["attempt_id"].as_str().unwrap();
    assert_eq!(
        project
            .attempt(
                "finish",
                "agent-a",
                token,
                &[
                    "--attempt-id",
                    attempt_id,
                    "--outcome",
                    "ready-to-verify",
                    "--note-code",
                    "source_edit"
                ]
            )
            .0,
        0
    );
    let pending_view = project.next();
    assert_eq!(pending_view["disposition"], "verification_required");
    assert_eq!(
        pending_view["repair_brief"]["history"]["awaiting_verification"],
        true
    );
    let verify = |project: &Project| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                project.task_id.as_str(),
                project.root.to_str().unwrap(),
                "--owner",
                "agent-a",
                "--lease-token",
                token,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let first = verify(&project);
    assert_eq!(first["event_persisted"], true);
    let run_id = first["native_scan"]["run_id"].as_str().unwrap();
    fs::remove_file(
        project
            .root
            .join(format!(".codeguard/reports/{run_id}.json")),
    )
    .unwrap();
    let (exit, pending) = project.attempt("start", "agent-a", token, &["--action-id", &action]);
    assert_eq!(exit, 3);
    assert_eq!(pending["reason"], "verification_required_before_retry");
    let second = verify(&project);
    assert_eq!(second["event_persisted"], true);
    assert_eq!(
        project
            .attempt("start", "agent-a", token, &["--action-id", &action])
            .0,
        0
    );
}

#[test]
fn concurrent_claims_have_one_owner() {
    let project = Project::new();
    let mut children = Vec::new();
    for owner in ["agent-a", "agent-b", "agent-c", "agent-d"] {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args([
                    "task",
                    "claim",
                    project.task_id.as_str(),
                    project.root.to_str().unwrap(),
                    "--owner",
                    owner,
                    "--format=json",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    let mut wins = 0;
    for child in children {
        let status = child.wait_with_output().unwrap().status;
        if status.code() == Some(0) {
            wins += 1;
        } else {
            assert_eq!(status.code(), Some(3));
        }
    }
    assert_eq!(wins, 1);
}

#[test]
fn expired_lease_can_be_reclaimed_without_reusing_the_old_token() {
    let project = Project::new();
    let (_, first) = project.run("claim", "agent-a", None);
    let old_token = first["lease_token"].as_str().unwrap();
    let state = project
        .root
        .join(format!(".codeguard/state/leases/{}.json", project.task_id));
    let mut lease: Value = serde_json::from_slice(&fs::read(&state).unwrap()).unwrap();
    lease["expires_at"] = json!(1);
    fs::write(&state, serde_json::to_vec_pretty(&lease).unwrap()).unwrap();
    let (exit, second) = project.run("claim", "agent-b", None);
    assert_eq!(exit, 0);
    assert_eq!(second["generation"], 2);
    assert_ne!(second["lease_token"], first["lease_token"]);
    assert_eq!(project.run("heartbeat", "agent-a", Some(old_token)).0, 3);
}

#[test]
fn missing_task_and_symlinked_lease_state_fail_without_claiming() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            &format!("CG-{}", "b".repeat(32)),
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(!project.root.join(".codeguard/state/task_locks").exists());

    let leases = project.root.join(".codeguard/state/leases");
    fs::create_dir(&leases).unwrap();
    let outside = project.root.join("protected.txt");
    fs::write(&outside, "do not change").unwrap();
    std::os::unix::fs::symlink(&outside, leases.join(format!("{}.json", project.task_id))).unwrap();
    let (exit, report) = project.run("claim", "agent-a", None);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "lease_invalid");
    assert_eq!(fs::read_to_string(outside).unwrap(), "do not change");
}

#[test]
fn held_lock_returns_incomplete_without_waiting_for_the_owner() {
    let project = Project::new();
    let locks = project.root.join(".codeguard/state/task_locks");
    fs::create_dir(&locks).unwrap();
    let _held = TaskFileLock::acquire(&locks.join(format!("{}.lock", project.task_id))).unwrap();
    let (exit, report) = project.run("claim", "agent-b", None);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "lease_lock_unavailable");
    assert!(
        !project
            .root
            .join(format!(".codeguard/state/leases/{}.json", project.task_id))
            .exists()
    );
}

#[test]
fn environment_blocker_can_be_claimed_for_repair() {
    let project = Project::new();
    let workspace: Value =
        serde_json::from_slice(&fs::read(project.root.join(".codeguard/workspace.json")).unwrap())
            .unwrap();
    let report = json!({
        "schema_version":"0.4.0", "report_type":"python_lint_feedback",
        "operation":"lint", "language":"python", "run_id":"lint-1-200",
        "workspace_binding":"bound", "workspace_id":workspace["workspace_id"],
        "command_status":"incomplete", "delivery_decision":"not_evaluated",
        "tool_approval":"unverified",
        "files":[{"path":"app.py", "run_status":"incomplete",
            "reason":"ruff_tool_not_found", "recheck_cwd":project.root,
            "recheck_argv":["ruff","check","app.py"], "findings":[]}]
    });
    fs::write(
        project.root.join(".codeguard/reports/lint-1-200.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "work",
            "sync",
            project.root.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(sync.status.code(), Some(3));
    let blocker_id = fs::read_dir(project.root.join(".codeguard/tasks"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .find(|name| name.starts_with("CG-B-"))
        .unwrap()
        .trim_end_matches(".md")
        .to_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            &blocker_id,
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let lease: Value = serde_json::from_slice(&output.stdout).unwrap();
    let token = lease["lease_token"].as_str().unwrap();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(action, "restore-checker-environment");
    let started = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "start",
            &blocker_id,
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--action-id",
            &action,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(started.status.code(), Some(0));
    let start: Value = serde_json::from_slice(&started.stdout).unwrap();
    let attempt_id = start["attempt_id"].as_str().unwrap();
    let finished = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "finish",
            &blocker_id,
            project.root.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--attempt-id",
            attempt_id,
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "tool_restored",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(finished.status.code(), Some(0));
    let finish: Value = serde_json::from_slice(&finished.stdout).unwrap();
    assert_eq!(finish["observed_change"], false);
    assert_eq!(
        project.next()["repair_brief"]["history"]["no_progress_count"],
        0
    );
}

#[test]
fn projection_recovery_keeps_live_lease_attempt_and_budget_unchanged() {
    fn snapshot(
        path: &std::path::Path,
        root: &std::path::Path,
        records: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                snapshot(&path, root, records);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                records.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let p = Project::new();
    let action = p.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, lease) = p.run("claim", "projection-owner", None);
    let token = lease["lease_token"].as_str().unwrap();
    assert_eq!(
        p.attempt(
            "start",
            "projection-owner",
            token,
            &["--action-id", &action]
        )
        .0,
        0
    );
    let history = p.next()["repair_brief"]["history"].clone();
    assert!(history["open_attempt_id"].is_string());
    let mut before = std::collections::BTreeMap::new();
    snapshot(&p.root.join(".codeguard/state"), &p.root, &mut before);
    snapshot(&p.root.join(".codeguard/findings"), &p.root, &mut before);
    fs::remove_file(p.root.join(format!(".codeguard/tasks/{}.md", p.task_id))).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync"])
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["restored_task_projections"], 1, "{response}");
    let mut after = std::collections::BTreeMap::new();
    snapshot(&p.root.join(".codeguard/state"), &p.root, &mut after);
    snapshot(&p.root.join(".codeguard/findings"), &p.root, &mut after);
    assert_eq!(after, before);
    let next = p.next();
    assert_eq!(next["repair_brief"]["history"], history);
    assert_eq!(next["repair_brief"]["disposition"], "waiting");
    assert_eq!(
        p.run("release", "projection-owner", Some(token)).1["reason"],
        "attempt_still_open"
    );
}
