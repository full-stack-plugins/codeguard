#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-check-audit-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='audit-test'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 42 }\n").unwrap();
        fs::write(root.join("Cargo.lock"), "version = 3\n[[package]]\nname='time'\nversion='0.1.40'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='0000000000000000000000000000000000000000000000000000000000000000'\n").unwrap();
        fs::create_dir(root.join("advisory-db")).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn native_tool(&self) -> PathBuf {
        let tool = self.0.join("cargo-audit-test");
        let result = json!({"database":{"advisory-count":1,"last-commit":null,"last-updated":null},"lockfile":{"dependency-count":1},"settings":{"target_arch":[],"target_os":[],"severity":null,"ignore":[],"informational_warnings":[]},"vulnerabilities":{"found":true,"count":1,"list":[{"advisory":{"id":"RUSTSEC-2020-0071","package":"time","aliases":["CVE-2020-26235"],"cvss":null},"package":{"name":"time","version":"0.1.40","source":"registry+https://github.com/rust-lang/crates.io-index","checksum":"0".repeat(64)}}]},"warnings":{}});
        fs::write(
            &tool,
            format!("#!/bin/sh\nprintf '%s\\n' '{result}'\nexit 1\n"),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, format: &str, extra: &[&str]) -> (i32, Vec<u8>) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
                self.0.to_str().unwrap(),
                "--format",
                format,
                "--jobs",
                "4",
                "--timeout",
                "10s",
            ])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        (output.status.code().unwrap(), output.stdout)
    }
    fn init(&self) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", self.0.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
    }
    fn cve(&self, tool: &std::path::Path) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "rust",
                self.0.to_str().unwrap(),
                "--cargo-audit-tool",
                tool.to_str().unwrap(),
                "--db",
                self.0.join("advisory-db").to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rust_cve_is_a_distinct_check_all_task_and_preserves_the_native_advisory() {
    let fixture = Fixture::new();
    let tool = fixture.native_tool();
    let (exit, bytes) = fixture.check(
        "json",
        &[
            "--cargo-audit-tool",
            tool.to_str().unwrap(),
            "--rustsec-db",
            fixture.0.join("advisory-db").to_str().unwrap(),
        ],
    );
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_cve"]["findings"][0]["advisory_id"],
        "RUSTSEC-2020-0071"
    );
    assert_eq!(
        report["native_results"]["rust_cve"]["database_freshness"],
        "unverified"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["id"] == "rust.cve")
    );
    let candidate = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["language"] == "rust" && row["category"] == "cve")
        .unwrap();
    assert_eq!(candidate["checker_id"], "rust.cargo_audit");
    assert_eq!(candidate["status"], "observed_unverified");
    let (sarif_exit, sarif_bytes) = fixture.check(
        "sarif",
        &[
            "--cargo-audit-tool",
            tool.to_str().unwrap(),
            "--rustsec-db",
            fixture.0.join("advisory-db").to_str().unwrap(),
        ],
    );
    assert_eq!(sarif_exit, 3);
    let sarif: Value = serde_json::from_slice(&sarif_bytes).unwrap();
    assert!(sarif["runs"][0]["results"].as_array().unwrap().iter().any(|entry| entry["properties"]["codeguardObservation"] == "native_finding_unverified"));
}

#[test]
fn missing_cargo_audit_is_a_visible_environment_blocker() {
    let fixture = Fixture::new();
    fixture.init();
    let (exit, bytes) = fixture.check("json", &[]);
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_cve"]["reason"],
        "cargo_audit_tool_not_selected"
    );
    assert_eq!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "rust" && row["category"] == "cve")
            .unwrap()["status"],
        "native_incomplete"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    let tasks: Vec<_> = fs::read_dir(fixture.0.join("codeguard/findings"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(tasks.iter().any(|path| {
        let fact: Value =
            serde_json::from_slice(&fs::read(path.join("finding.json")).unwrap()).unwrap();
        fact["checker_id"] == "rust.cargo_audit" && fact["kind"] == "blocker"
    }));
}

#[test]
fn initialized_rust_cve_reuses_one_task_and_native_recheck_keeps_it_open() {
    let fixture = Fixture::new();
    fixture.init();
    let tool = fixture.native_tool();
    for _ in 0..2 {
        let report = fixture.cve(&tool);
        assert_eq!(report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071");
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
    let tasks: Vec<_> = fs::read_dir(fixture.0.join("codeguard/findings"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(tasks.len(), 1, "同一 CVE 义务应只有一张稳定任务");
    let fact: Value =
        serde_json::from_slice(&fs::read(tasks[0].join("finding.json")).unwrap()).unwrap();
    assert_eq!(fact["checker_id"], "rust.cargo_audit");
    assert_eq!(fact["state"], "open");
    let id = fact["id"].as_str().unwrap();
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let brief: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        brief["repair_brief"]["checker_id"], "rust.cargo_audit",
        "{brief}"
    );
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-audit-tool",
            tool.to_str().unwrap(),
            "--rustsec-db",
            fixture.0.join("advisory-db").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(result["observation"], "still_blocked", "{result}");
    assert_eq!(result["event_persisted"], true, "{result}");
    let fact_after: Value =
        serde_json::from_slice(&fs::read(tasks[0].join("finding.json")).unwrap()).unwrap();
    assert_eq!(fact_after["state"], "open");
    let next_after_verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let brief_after_verify: Value = serde_json::from_slice(&next_after_verify.stdout).unwrap();
    assert_eq!(
        brief_after_verify["repair_brief"]["checker_id"], "rust.cargo_audit",
        "{brief_after_verify}"
    );
    let (check_exit, check_bytes) = fixture.check(
        "json",
        &[
            "--cargo-audit-tool",
            tool.to_str().unwrap(),
            "--rustsec-db",
            fixture.0.join("advisory-db").to_str().unwrap(),
        ],
    );
    assert_eq!(check_exit, 3);
    let check_report: Value = serde_json::from_slice(&check_bytes).unwrap();
    assert_eq!(check_report["delivery_decision"], "incomplete");
    assert_eq!(
        check_report["native_results"]["rust_cve"]["findings"][0]["advisory_id"],
        "RUSTSEC-2020-0071"
    );
    let cve_tasks = fs::read_dir(fixture.0.join("codeguard/findings"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path().join("finding.json");
            let fact: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
            (fact["checker_id"] == "rust.cargo_audit").then_some(fact)
        })
        .collect::<Vec<_>>();
    assert_eq!(cve_tasks.len(), 1);
    assert_eq!(cve_tasks[0]["id"], id);
}

#[test]
fn rust_cve_attempt_is_bound_to_its_native_recheck_history() {
    let fixture = Fixture::new();
    fixture.init();
    let tool = fixture.native_tool();
    fixture.cve(&tool);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    let id = next["repair_brief"]["task_id"].as_str().unwrap();
    let action = next["repair_brief"]["action_id"].as_str().unwrap();
    let claim = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "claim",
            id,
            fixture.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(claim.status.code(), Some(0));
    let claim: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let token = claim["lease_token"].as_str().unwrap();
    let start = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "start",
            id,
            fixture.0.to_str().unwrap(),
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
    let start: Value = serde_json::from_slice(&start.stdout).unwrap();
    let attempt_id = start["attempt_id"].as_str().unwrap();
    let finish = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "finish",
            id,
            fixture.0.to_str().unwrap(),
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
    assert_eq!(finish.status.code(), Some(0));
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--cargo-audit-tool",
            tool.to_str().unwrap(),
            "--rustsec-db",
            fixture.0.join("advisory-db").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verify["observation"], "still_blocked", "{verify}");
    assert_eq!(verify["event_persisted"], true, "{verify}");
    let after = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let after: Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(
        after["repair_brief"]["checker_id"], "rust.cargo_audit",
        "{after}"
    );
    assert_eq!(
        after["repair_brief"]["history"]["awaiting_verification"],
        false
    );
    assert_eq!(
        after["repair_brief"]["verification_observation"],
        "still_blocked"
    );
    assert_eq!(after["repair_brief"]["history"]["no_progress_count"], 1);
    let action = after["repair_brief"]["action_id"].as_str().unwrap();
    for _ in 0..1 {
        let start = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "attempt",
                "start",
                id,
                fixture.0.to_str().unwrap(),
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
        assert_eq!(
            start.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&start.stdout)
        );
        let start: Value = serde_json::from_slice(&start.stdout).unwrap();
        let finish = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "attempt",
                "finish",
                id,
                fixture.0.to_str().unwrap(),
                "--owner",
                "agent-a",
                "--lease-token",
                token,
                "--attempt-id",
                start["attempt_id"].as_str().unwrap(),
                "--outcome",
                "no-change",
                "--note-code",
                "no_change",
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(finish.status.code(), Some(0));
    }
    let stopped = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let stopped: Value = serde_json::from_slice(&stopped.stdout).unwrap();
    assert_eq!(stopped["disposition"], "needs_decision", "{stopped}");
    assert!(
        stopped["repair_brief"]["history"]["no_progress_count"]
            .as_u64()
            .unwrap()
            >= 2
    );
    let repeated = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "attempt",
            "start",
            id,
            fixture.0.to_str().unwrap(),
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
    assert_eq!(repeated.status.code(), Some(3));
    let repeated: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(repeated["reason"], "no_progress_budget_exhausted");
}

#[test]
fn input_change_after_attempt_cannot_bind_a_new_cve_recheck_to_it() {
    let fixture = Fixture::new();
    fixture.init();
    let tool = fixture.native_tool();
    fixture.cve(&tool);
    let root = fixture.0.to_str().unwrap();
    let run = |args: &[&str]| -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    };
    let (_, next) = run(&["next", root, "--format=json"]);
    let id = next["repair_brief"]["task_id"].as_str().unwrap();
    let action = next["repair_brief"]["action_id"].as_str().unwrap();
    let (code, claim) = run(&[
        "task",
        "claim",
        id,
        root,
        "--owner",
        "agent-a",
        "--format=json",
    ]);
    assert_eq!(code, 0);
    let token = claim["lease_token"].as_str().unwrap();
    let (code, started) = run(&[
        "task",
        "attempt",
        "start",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        action,
        "--format=json",
    ]);
    assert_eq!(code, 0);
    let (code, _) = run(&[
        "task",
        "attempt",
        "finish",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--attempt-id",
        started["attempt_id"].as_str().unwrap(),
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "tool_restored",
        "--format=json",
    ]);
    assert_eq!(code, 0);
    let lock = fixture.0.join("Cargo.lock");
    let mut bytes = fs::read(&lock).unwrap();
    bytes.extend_from_slice(b"# changed after attempt\n");
    fs::write(&lock, bytes).unwrap();
    let (_, verified) = run(&[
        "task",
        "verify",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--cargo-audit-tool",
        tool.to_str().unwrap(),
        "--rustsec-db",
        fixture.0.join("advisory-db").to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(verified["event_persisted"], true, "{verified}");
    let event_path = fixture.0.join(format!(
        "codeguard/findings/{id}/events/verify-{}.json",
        verified["native_scan"]["run_id"].as_str().unwrap()
    ));
    let event: Value = serde_json::from_slice(&fs::read(event_path).unwrap()).unwrap();
    assert!(event["attempt_id"].is_null(), "{event}");
    let (_, next) = run(&["next", root, "--format=json"]);
    assert_eq!(
        next["repair_brief"]["history"]["awaiting_verification"],
        false
    );
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 0);
    let (code, new_attempt) = run(&[
        "task",
        "attempt",
        "start",
        id,
        root,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        next["repair_brief"]["action_id"].as_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(code, 0, "{new_attempt}");
}
