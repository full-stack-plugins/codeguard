#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-task-verify-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), "import os\n").unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        Self(root)
    }

    fn lint(&self, tool: Option<&str>) -> Value {
        let mut args = vec!["lint", "python", self.0.to_str().unwrap()];
        if let Some(tool) = tool {
            args.extend(["--ruff-tool", tool]);
        }
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn verify(&self, id: &str, tool: Option<&str>) -> (i32, Value) {
        self.verify_with_lease(id, tool, None, None)
    }

    fn verify_with_lease(
        &self,
        id: &str,
        tool: Option<&str>,
        owner: Option<&str>,
        token: Option<&str>,
    ) -> (i32, Value) {
        let mut args = vec!["task", "verify", id, self.0.to_str().unwrap()];
        if let Some(tool) = tool {
            args.extend(["--ruff-tool", tool]);
        }
        if let (Some(owner), Some(token)) = (owner, token) {
            args.extend(["--owner", owner, "--lease-token", token]);
        }
        args.push("--format=json");
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn one_task(&self) -> String {
        let tasks = fs::read_dir(self.0.join(".codeguard/tasks"))
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .trim_end_matches(".md")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(tasks.len(), 1);
        tasks[0].clone()
    }

    fn fact(&self, id: &str) -> Value {
        serde_json::from_slice(
            &fs::read(
                self.0
                    .join(format!(".codeguard/findings/{id}/finding.json")),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn next(&self) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", self.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

#[test]
fn invalid_verify_budget_is_rejected_before_leasing_or_native_execution() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    let marker = project.0.join("native-started");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for budget in ["0ms", "-1s", "25h", "unknown"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                &id,
                project.0.to_str().unwrap(),
                "--timeout",
                budget,
                "--ruff-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{budget}");
        assert!(output.stdout.is_empty());
        assert!(!marker.exists());
        assert!(
            !project
                .0
                .join(format!(".codeguard/state/leases/{id}.json"))
                .exists()
        );
    }
    let invalid_environment = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("CODEGUARD_TIMEOUT", "unlimited")
        .output()
        .unwrap();
    assert_eq!(invalid_environment.status.code(), Some(2));
    assert!(!marker.exists());
    assert!(
        !project
            .0
            .join(format!(".codeguard/state/leases/{id}.json"))
            .exists()
    );
}

#[test]
fn verify_deadline_keeps_task_open_and_does_not_persist_incomplete_resolution() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = project.0.join("ruff");
    fs::write(&tool, "#!/bin/sh\nsleep 2\nprintf 'ruff 0.16.8\\n'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let events = project.0.join(format!(".codeguard/findings/{id}/events"));
    let before = fs::read_dir(&events).unwrap().count();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            project.0.to_str().unwrap(),
            "--timeout",
            "100ms",
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(started.elapsed() < Duration::from_secs(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["reason"], "request_deadline_exceeded");
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["execution_budget"]["source"], "cli");
    assert_eq!(
        report["execution_budget"]["enforcement"],
        "native_execution_only"
    );
    assert_eq!(report["observation"], "incomplete");
    assert_eq!(report["event_persisted"], false);
    assert_eq!(
        report["native_scan"]["files"][0]["reason"],
        "request_deadline_exceeded"
    );
    assert_eq!(fs::read_dir(&events).unwrap().count(), before);
    let lease: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/state/leases/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(lease["status"], "released");
    assert_eq!(project.next()["repair_brief"]["task_id"], id);
}

#[test]
fn verify_without_existing_owner_uses_and_releases_its_own_lease() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    let (_, report) = project.verify(&id, None);
    assert_eq!(report["event_persisted"], true);
    let lease: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/state/leases/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(lease["status"], "released");
    assert!(
        lease["owner"]
            .as_str()
            .unwrap()
            .starts_with("codeguard-verify-")
    );
}

#[test]
fn verify_reads_project_runtime_default_and_keeps_task_open() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s"}"#,
    )
    .unwrap();
    let (exit, report) = project.verify(&id, None);
    assert_eq!(exit, 3);
    assert_eq!(report["execution_budget"]["timeout_ms"], 2000);
    assert_eq!(report["execution_budget"]["source"], "project_default");
    assert_eq!(project.fact(&id)["state"], "open");
}

#[test]
fn borrowed_verify_keeps_caller_lease_and_wrong_token_never_starts_scan() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    let claim = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            &id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claim.status.code(), Some(0));
    let receipt: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let token = receipt["lease_token"].as_str().unwrap();
    let reports_before = fs::read_dir(project.0.join(".codeguard/reports"))
        .unwrap()
        .count();
    let (unclaimed_exit, unclaimed) = project.verify(&id, None);
    assert_eq!(unclaimed_exit, 3);
    assert_eq!(unclaimed["reason"], "task_already_claimed");
    assert!(unclaimed["native_scan"].is_null());
    let (exit, denied) =
        project.verify_with_lease(&id, None, Some("agent-a"), Some(&"0".repeat(64)));
    assert_eq!(exit, 3);
    assert_eq!(denied["reason"], "lease_token_mismatch");
    assert!(denied["native_scan"].is_null());
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        reports_before
    );
    let (_, observed) = project.verify_with_lease(&id, None, Some("agent-a"), Some(token));
    assert_eq!(observed["event_persisted"], true);
    let lease: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/state/leases/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(lease["status"], "active");
    assert_eq!(lease["owner"], "agent-a");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn native_ruff_verify_respects_the_borrowed_token_before_scanning() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    let claim = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claim.status.code(), Some(0));
    let receipt: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let token = receipt["lease_token"].as_str().unwrap();
    let reports_before = fs::read_dir(project.0.join(".codeguard/reports"))
        .unwrap()
        .count();
    let (_, denied) =
        project.verify_with_lease(id, Some(&tool), Some("agent-a"), Some(&"0".repeat(64)));
    assert_eq!(denied["reason"], "lease_token_mismatch");
    assert!(denied["native_scan"].is_null());
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/reports"))
            .unwrap()
            .count(),
        reports_before
    );
    let (_, verified) = project.verify_with_lease(id, Some(&tool), Some("agent-a"), Some(token));
    assert_eq!(verified["event_persisted"], true);
    assert_eq!(verified["observation"], "still_present");
    let lease: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/state/leases/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(lease["status"], "active");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn native_recheck_of_a_repaired_finding_clears_pending_attempt_without_closing_the_task() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    let action = project.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let claim = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claim.status.code(), Some(0));
    let receipt: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let token = receipt["lease_token"].as_str().unwrap();
    let start = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "start",
            id,
            project.0.to_str().unwrap(),
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
    assert_eq!(start.status.code(), Some(0));
    let started: Value = serde_json::from_slice(&start.stdout).unwrap();
    let attempt_id = started["attempt_id"].as_str().unwrap();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let finish = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "finish",
            id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--attempt-id",
            attempt_id,
            "--outcome",
            "ready-to-verify",
            "--note-code",
            "source_edit",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(finish.status.code(), Some(0));
    assert_eq!(
        project.next()["repair_brief"]["history"]["awaiting_verification"],
        true
    );
    let (_, verified) = project.verify_with_lease(id, Some(&tool), Some("agent-a"), Some(token));
    assert_eq!(
        verified["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(verified["event_persisted"], true);
    let next = project.next();
    assert_eq!(
        next["repair_brief"]["history"]["awaiting_verification"],
        false
    );
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 0);
    assert_eq!(next["disposition"], "verification_required");
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(
        next["repair_brief"]["verification_run_id"],
        verified["native_scan"]["run_id"]
    );
    fs::write(project.0.join("app.py"), "pass\n# changed again\n").unwrap();
    let stale = project.next();
    assert!(stale["repair_brief"]["verification_observation"].is_null());
    assert_eq!(stale["disposition"], "verification_required");
    assert_eq!(project.fact(id)["state"], "open");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn changed_source_with_the_same_native_finding_becomes_actionable_after_recheck() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    fs::write(
        project.0.join("app.py"),
        "import os\n# changed but F401 remains\n",
    )
    .unwrap();
    assert_eq!(project.next()["disposition"], "verification_required");
    let (_, verified) = project.verify(id, Some(&tool));
    assert_eq!(verified["observation"], "still_present");
    let next = project.next();
    assert_eq!(next["disposition"], "actionable");
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "still_present"
    );
    assert_eq!(
        next["repair_brief"]["source_sha256"],
        verified["native_scan"]["files"][0]["source_sha256"]
    );
    fs::write(
        project.0.join("app.py"),
        "import os\n# changed again after verification\n",
    )
    .unwrap();
    assert_eq!(project.next()["disposition"], "verification_required");
    fs::write(
        project.0.join("app.py"),
        "import os\n# changed but F401 remains\n",
    )
    .unwrap();
    let action = next["repair_brief"]["action_id"].as_str().unwrap();
    let claim = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claim.status.code(), Some(0));
    let receipt: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let token = receipt["lease_token"].as_str().unwrap();
    let start = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "start",
            id,
            project.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--action-id",
            action,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(start.status.code(), Some(0));
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_config_remains_blocked_and_records_native_recheck_without_closure() {
    let project = Project::new();
    assert_eq!(project.lint(None)["backlog_sync"]["new_blockers"], 1);
    let id = project.one_task();
    let (exit, verification) = project.verify(&id, None);
    assert_eq!(exit, 3);
    assert_eq!(verification["observation"], "still_blocked");
    assert_eq!(
        verification["native_scan"]["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert_eq!(verification["event_persisted"], true);
    assert_eq!(verification["delivery_decision"], "not_evaluated");
    assert_eq!(project.fact(&id)["state"], "open");
    let events = fs::read_dir(project.0.join(format!(".codeguard/findings/{id}/events")))
        .unwrap()
        .count();
    assert_eq!(events, 2); // 初见与复检观察；再次扫描只写本地证据。
    assert_eq!(
        fs::read_dir(
            project
                .0
                .join(format!(".codeguard/state/observations/{id}"))
        )
        .unwrap()
        .count(),
        2
    );
}

#[test]
fn invalid_or_missing_task_does_not_start_a_native_scan() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            "CG-invalid",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        fs::read_dir(project.0.join(".codeguard/reports"))
            .unwrap()
            .next()
            .is_none()
    );
    let id = format!("CG-B-{}", "a".repeat(32));
    let (exit, result) = project.verify(&id, None);
    assert_eq!(exit, 3);
    assert!(result["native_scan"].is_null());
    assert_eq!(result["reason"], "task_record_unavailable");
    assert_eq!(result["execution_budget"]["timeout_ms"], 1_800_000);
    assert_eq!(result["execution_budget"]["source"], "builtin_default");
    assert!(
        fs::read_dir(project.0.join(".codeguard/reports"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn forged_recovery_event_cannot_suppress_an_actual_blocker_in_next() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    assert_eq!(project.verify(&id, None).1["observation"], "still_blocked");
    let event = fs::read_dir(project.0.join(format!(".codeguard/findings/{id}/events")))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("verify-")
        })
        .unwrap();
    let mut value: Value = serde_json::from_slice(&fs::read(&event).unwrap()).unwrap();
    value["observation"] = serde_json::json!("environment_restored_unverified_policy");
    fs::write(&event, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(next["reason"], "verification_event_invalid");
    assert_eq!(next["delivery_decision"], "not_evaluated");
}

#[test]
fn missing_local_recheck_report_does_not_reuse_an_unverifiable_recovery_claim() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    let verification = project.verify(&id, None).1;
    let run_id = verification["native_scan"]["run_id"].as_str().unwrap();
    fs::remove_file(project.0.join(format!(".codeguard/reports/{run_id}.json"))).unwrap();
    let next = project.next();
    assert_eq!(next["disposition"], "needs_decision");
    assert_eq!(
        next["repair_brief"]["reason_code"],
        "project_ruff_config_not_found"
    );
    assert_eq!(next["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn recovered_environment_is_observed_but_old_blocker_is_not_falsely_closed() {
    let project = Project::new();
    project.lint(None);
    let id = project.one_task();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, verification) = project.verify(&id, Some(&tool));
    assert_eq!(exit, 3);
    assert_eq!(
        verification["observation"],
        "environment_restored_unverified_policy"
    );
    assert_eq!(
        verification["native_scan"]["files"][0]["run_status"],
        "findings"
    );
    assert_eq!(
        verification["native_scan"]["files"][0]["findings"][0]["rule_id"],
        "F401"
    );
    assert_eq!(verification["event_persisted"], true);
    assert_eq!(project.fact(&id)["state"], "open");
    let next = project.next();
    assert_eq!(next["disposition"], "actionable");
    assert_eq!(next["repair_brief"]["native_rule_id"], "F401");
    assert_ne!(next["repair_brief"]["task_id"], id);
    fs::remove_file(project.0.join("ruff.toml")).unwrap();
    project.lint(None);
    assert_eq!(project.next()["disposition"], "needs_decision");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn absent_original_finding_is_only_a_candidate_until_policy_and_coverage_are_verified() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let (exit, verification) = project.verify(id, Some(&tool));
    assert_eq!(exit, 3);
    assert_eq!(
        verification["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(
        verification["native_scan"]["files"][0]["run_status"],
        "passed"
    );
    assert_eq!(verification["event_persisted"], true);
    assert_eq!(project.fact(id)["state"], "open");
    assert_eq!(verification["delivery_decision"], "not_evaluated");
    assert_eq!(project.next()["disposition"], "verification_required");
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    project.lint(Some(&tool));
    assert_eq!(project.next()["disposition"], "actionable");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn d100_check_all_creates_a_stable_task_and_rechecks_with_native_ruff() {
    let project = Project::new();
    fs::write(
        project.0.join("app.py"),
        "def calculate():\n    return 42\n",
    )
    .unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['D100']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(initial.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&initial.stdout).unwrap();
    assert_eq!(
        report["native_results"]["python_lint"]["backlog_status"],
        "synced_partial"
    );
    let id = project.one_task();
    let next = project.next();
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(next["repair_brief"]["native_rule_id"], "D100");
    assert_eq!(next["disposition"], "actionable");

    let (_, still_present) = project.verify(&id, Some(&tool));
    assert_eq!(still_present["observation"], "still_present");
    fs::write(
        project.0.join("app.py"),
        "\"\"\"Math helpers.\"\"\"\n\ndef calculate():\n    return 42\n",
    )
    .unwrap();
    let (_, rechecked) = project.verify(&id, Some(&tool));
    assert_eq!(
        rechecked["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(rechecked["event_persisted"], true);
    assert_eq!(project.fact(&id)["state"], "open");
    assert_eq!(project.next()["disposition"], "verification_required");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn d101_task_explains_class_docstring_and_requires_native_recheck() {
    let project = Project::new();
    fs::write(
        project.0.join("app.py"),
        "\"\"\"Public module.\"\"\"\n\nclass PublicType:\n    pass\n",
    )
    .unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['D101']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(initial.status.code(), Some(3));
    let id = project.one_task();
    let task =
        fs::read_to_string(project.0.join(".codeguard/tasks").join(format!("{id}.md"))).unwrap();
    assert!(task.contains("Ruff 规则 `D101`"));
    assert!(task.contains("确认该公共类的职责，为类补充准确的 docstring。"));
    assert_eq!(project.next()["repair_brief"]["native_rule_id"], "D101");

    fs::write(
        project.0.join(".codeguard/decisions/forged.json"),
        format!("{{\"finding_id\":\"{id}\",\"native_rule_id\":\"D101\",\"approved\":true}}"),
    )
    .unwrap();
    let self_approved = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(self_approved.status.code(), Some(3));
    let after_self_approval: Value = serde_json::from_slice(&self_approved.stdout).unwrap();
    assert_eq!(after_self_approval["delivery_decision"], "incomplete");
    assert!(
        after_self_approval["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|file| file["findings"].as_array().into_iter().flatten())
            .any(|finding| finding["rule_id"] == "D101" && finding["finding_id"] == id)
    );
    assert_eq!(project.one_task(), id);
    assert_eq!(project.fact(&id)["state"], "open");
    let proposed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "rules",
            "whitelist",
            "propose",
            &id,
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(proposed.status.code(), Some(3));
    let proposal: Value = serde_json::from_slice(&proposed.stdout).unwrap();
    assert!(proposal["candidate"].is_null());
    assert_eq!(proposal["authority"], "unverified");
    assert_eq!(proposal["gate_effect"], "none");
    assert_eq!(proposal["status"], "rulepack_mapping_unavailable");
    assert!(
        proposal["missing_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "approved_rulepack_identity")
    );
    assert!(
        proposal["missing_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "reviewed_native_rule_mapping")
    );
    assert!(
        proposal["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "review_native_rule_and_tool_version")
    );
    let human_proposal = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "rules",
            "whitelist",
            "propose",
            &id,
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(human_proposal.status.code(), Some(3));
    assert!(
        String::from_utf8(human_proposal.stdout)
            .unwrap()
            .contains("reviewed_native_rule_mapping")
    );

    fs::write(
        project.0.join("app.py"),
        "\"\"\"Public module.\"\"\"\n\nclass PublicType:  # noqa: D101\n    pass\n",
    )
    .unwrap();
    let (_, suppressed) = project.verify(&id, Some(&tool));
    assert_eq!(suppressed["observation"], "suppression_requires_review");
    assert_eq!(project.fact(&id)["state"], "open");

    fs::write(
        project.0.join("app.py"),
        "\"\"\"Public module.\"\"\"\n\nclass PublicType:\n    \"\"\"Public class.\"\"\"\n",
    )
    .unwrap();
    let (_, rechecked) = project.verify(&id, Some(&tool));
    assert_eq!(
        rechecked["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(project.fact(&id)["state"], "open");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn adding_noqa_does_not_look_like_a_repaired_finding() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    fs::write(project.0.join("app.py"), "import os  # noqa: F401\n").unwrap();
    let (exit, verification) = project.verify(id, Some(&tool));
    assert_eq!(exit, 3);
    assert_eq!(verification["schema_version"], "0.9.0");
    assert_eq!(verification["observation"], "suppression_requires_review");
    assert_eq!(
        verification["native_scan"]["files"][0]["run_status"],
        "suppressed"
    );
    assert_eq!(
        verification["native_scan"]["files"][0]["suppression_audit"]["suppressed_rule_ids"],
        serde_json::json!(["F401"])
    );
    assert_eq!(verification["event_persisted"], true);
    assert_eq!(project.fact(id)["state"], "open");
    let next = project.next();
    assert_eq!(next["disposition"], "needs_decision");
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "suppression_requires_review"
    );
    assert_eq!(next["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn disabling_original_rule_does_not_look_like_a_repaired_finding() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['E501']\n").unwrap();
    let (_, verification) = project.verify(id, Some(&tool));
    assert_eq!(verification["observation"], "rule_coverage_requires_review");
    assert_eq!(
        verification["native_scan"]["files"][0]["run_status"],
        "passed"
    );
    assert_eq!(project.fact(id)["state"], "open");
    let next = project.next();
    assert_eq!(next["disposition"], "needs_decision");
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "rule_coverage_requires_review"
    );
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn per_file_ignore_requires_review_before_original_task_can_close() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let initial = project.lint(Some(&tool));
    let id = initial["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    fs::write(
        project.0.join("ruff.toml"),
        "[lint]\nselect = ['F401']\nper-file-ignores = { 'app.py' = ['F401'] }\n",
    )
    .unwrap();
    let (_, verification) = project.verify(id, Some(&tool));
    assert_eq!(verification["observation"], "suppression_requires_review");
    assert_eq!(
        verification["native_scan"]["files"][0]["run_status"],
        "passed"
    );
    assert_eq!(
        verification["native_scan"]["files"][0]["rule_settings"]["per_file_ignores_present"],
        true
    );
    assert_eq!(project.fact(id)["state"], "open");
    assert_eq!(project.next()["disposition"], "needs_decision");
}

#[test]
fn verification_schemas_cannot_claim_quality_allow_or_resolution() {
    let preview: Value = serde_json::from_str(include_str!(
        "../../../schemas/task-verification-preview.schema.json"
    ))
    .unwrap();
    let npm: Value = serde_json::from_str(include_str!(
        "../../../schemas/npm-cve-workbench-observation.schema.json"
    ))
    .unwrap();
    assert_eq!(preview["$defs"]["npmWorkbenchObservation"], npm);

    for source in [
        include_str!("../../../schemas/task-verification-preview.schema.json"),
        include_str!("../../../schemas/task-verification-event.schema.json"),
    ] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert!(!source.contains("\"allow\""));
        assert!(!source.contains("\"resolved\""));
    }
}
