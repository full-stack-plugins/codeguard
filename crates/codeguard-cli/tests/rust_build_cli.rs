#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn init(&self) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", self.0.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
    }
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cg-build-cli-{}-{nonce}-{}",
            std::process::id(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let root = root.canonicalize().unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='build-cli'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname=\"build-cli\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn answer() -> i32 { \"wrong\" }\n",
        )
        .unwrap();
        Self(root)
    }
    fn tool(&self, body: &str) -> PathBuf {
        let tool = self.0.join("cargo-probe");
        fs::write(&tool, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["build", "rust", self.0.to_str().unwrap(), "--format=json"])
            .args(args)
            .env(
                "PATH",
                if args.contains(&"--cargo-tool") {
                    std::env::var_os("PATH").unwrap_or_default()
                } else {
                    std::ffi::OsString::new()
                },
            )
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).expect("构建入口返回JSON"),
        )
    }
    fn verify(&self, task_id: &str, tool: &std::path::Path) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                task_id,
                self.0.to_str().unwrap(),
                "--cargo-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).expect("任务复检返回JSON"),
        )
    }
    fn error(&self) -> String {
        json!({"reason":"compiler-message","package_id":"path+file:///example#build-cli@0.1.0","manifest_path":self.0.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":self.0.join("src/lib.rs")},"message":{"level":"error","code":{"code":"E0308"},"spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":25,"byte_start":24,"byte_end":31,"is_primary":true}]}}).to_string()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_type_error_preserves_attributed_diagnostic_without_claiming_test_or_delivery_success() {
    let fixture = Fixture::new();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nexit 101",
        fixture.error()
    ));
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert_eq!(report["build_success"], false);
    assert_eq!(report["test_execution"], false);
    assert_eq!(report["build_level"], "type_check");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["findings"][0]["rule_id"], "E0308");
    assert_eq!(report["findings"][0]["path"], "src/lib.rs");
    assert_eq!(
        report["findings"][0]["repair_brief"]["status"],
        "repair_guidance"
    );
}

#[test]
fn missing_tool_lock_bad_stream_and_environment_failure_stay_incomplete() {
    let fixture = Fixture::new();
    let (_, report) = fixture.check(&[]);
    assert_eq!(report["reason"], "cargo_tool_not_selected");
    let tool = fixture
        .tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":false}'\nexit 101");
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["local_scan_complete"], false);
    assert!(report["findings"].as_array().unwrap().is_empty());
    fs::remove_file(fixture.0.join("Cargo.lock")).unwrap();
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "locked_cargo_inputs_unavailable");
    assert!(!fixture.0.join("Cargo.lock").exists());
}

#[test]
fn input_changes_remove_source_repair_permission_and_timeout_is_bounded() {
    let fixture = Fixture::new();
    let tool=fixture.tool(&format!("printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nprintf 'changed' > src/lib.rs\nexit 101",fixture.error()));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "inputs_changed_during_scan");
    assert_eq!(
        report["findings"][0]["repair_brief"]["allowed_paths"],
        json!([])
    );
    let tool = fixture.tool("sleep 2");
    let (_, report) =
        fixture.check(&["--cargo-tool", tool.to_str().unwrap(), "--timeout", "100ms"]);
    assert_eq!(report["reason"], "request_deadline_exceeded");
}

#[test]
fn invalid_tool_argument_is_rejected_before_execution() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "build",
            "rust",
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            "cargo",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn out_of_scope_diagnostic_and_tool_changes_are_not_repair_authority() {
    let fixture = Fixture::new();
    let bad = fixture.error().replace("src/lib.rs", "outside.rs");
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nexit 101",
        bad
    ));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "native_target_outside_observed_scope");
    assert!(report["findings"].as_array().unwrap().is_empty());
    let tool=fixture.tool(&format!("printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nprintf '\\n# changed' >> \"$0\"\nexit 101",fixture.error()));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["reason"], "tool_changed_during_scan");
    assert_eq!(
        report["findings"][0]["repair_brief"]["allowed_paths"],
        json!([])
    );
}

#[test]
#[ignore = "requires explicit existing CODEGUARD_CARGO_BIN; actual public Cargo build probe"]
fn actual_public_build_rechecks_compiler_error_and_reports_clean_type_check_only() {
    let cargo = std::env::var("CODEGUARD_CARGO_BIN").expect("既有Cargo路径");
    let fixture = Fixture::new();
    let (exit, bad) = fixture.check(&["--cargo-tool", &cargo]);
    assert_eq!(exit, 3);
    assert_eq!(bad["local_scan_complete"], true, "{bad}");
    assert_eq!(bad["build_success"], false);
    assert_eq!(bad["findings"][0]["rule_id"], "E0308");
    assert!(bad["tool_sha256"].as_str().is_some());
    fs::write(fixture.0.join("src/lib.rs"),"pub fn answer() -> i32 { 42 }\n#[test] fn not_run() { panic!(\"not executed by check\"); }\n").unwrap();
    let (_, clean) = fixture.check(&["--cargo-tool", &cargo]);
    assert_eq!(clean["local_scan_complete"], true, "{clean}");
    assert_eq!(clean["build_success"], true);
    assert_eq!(clean["test_execution"], false);
    assert_eq!(clean["delivery_decision"], "not_evaluated");
    assert!(clean["findings"].as_array().unwrap().is_empty());
    fs::write(
        fixture.0.join("build.rs"),
        "fn main() { panic!(\"environment fixture failed\"); }\n",
    )
    .unwrap();
    let (_, blocked) = fixture.check(&["--cargo-tool", &cargo]);
    assert_eq!(blocked["local_scan_complete"], false);
    assert!(blocked["findings"].as_array().unwrap().is_empty());
}

#[test]
#[ignore = "requires explicit existing CODEGUARD_CARGO_BIN; actual Cargo task verify"]
fn actual_cargo_task_verify_keeps_clean_type_check_as_unverified_candidate() {
    let cargo = std::env::var("CODEGUARD_CARGO_BIN").expect("既有Cargo路径");
    let fixture = Fixture::new();
    fixture.init();
    let (_, initial) = fixture.check(&["--cargo-tool", &cargo]);
    assert_eq!(initial["backlog_status"], "synced", "{initial}");
    let task_id = initial["findings"][0]["finding_id"].as_str().unwrap();
    let (_, present) = fixture.verify(task_id, std::path::Path::new(&cargo));
    assert_eq!(present["observation"], "still_present", "{present}");
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    let (_, absent) = fixture.verify(task_id, std::path::Path::new(&cargo));
    assert_eq!(
        absent["observation"], "candidate_absent_unverified_policy",
        "{absent}"
    );
    assert_eq!(absent["event_persisted"], true);
    assert_eq!(
        absent["native_scan"]["normal_scan"]["test_execution"],
        false
    );
}

#[test]
fn cancellation_takes_priority_over_changed_inputs_and_removes_repair_permission() {
    let fixture = Fixture::new();
    let tool=fixture.tool(&format!("printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nprintf changed > src/lib.rs\ntouch ready\nsleep 20",fixture.error()));
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "build",
            "rust",
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !fixture.0.join("ready").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(fixture.0.join("ready").exists());
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["reason"], "request_cancelled");
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(
        report["findings"][0]["repair_brief"]["allowed_paths"],
        json!([])
    );
}

#[test]
fn initialized_build_scans_sync_one_stable_task_and_next_uses_original_build_checker() {
    let fixture = Fixture::new();
    fixture.init();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nexit 101",
        fixture.error()
    ));
    for _ in 0..2 {
        let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
        assert_eq!(report["backlog_status"], "synced", "{report}");
    }
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["checker_id"], "rust.cargo_check",
        "{next}"
    );
    assert!(
        next["repair_brief"]["recheck_argv"]
            .to_string()
            .contains("verify")
    );
    let task = fs::read_dir(fixture.0.join(".codeguard/tasks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(fs::read_to_string(task).unwrap().contains("原生编译"));
}

#[test]
fn build_task_verify_rechecks_native_error_and_keeps_zero_diagnostic_unverified() {
    let fixture = Fixture::new();
    fixture.init();
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nexit 101",
        fixture.error()
    ));
    let (_, initial) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(initial["backlog_status"], "synced", "{initial}");
    let task_id = initial["findings"][0]["finding_id"].as_str().unwrap();
    let (exit, present) = fixture.verify(task_id, &tool);
    assert_eq!(exit, 3);
    assert_eq!(present["observation"], "still_present", "{present}");
    assert_eq!(present["schema_version"], "0.8.0");
    assert_eq!(present["event_persisted"], true);
    assert_eq!(
        present["native_scan"]["report_type"],
        "rust_build_task_recheck"
    );
    assert_eq!(
        present["native_scan"]["normal_scan"]["findings"][0]["finding_id"],
        task_id
    );
    let clean =
        fixture.tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'\nexit 0");
    let (_, absent) = fixture.verify(task_id, &clean);
    assert_eq!(
        absent["observation"], "candidate_absent_unverified_policy",
        "{absent}"
    );
    assert_eq!(absent["event_persisted"], true);
    assert_eq!(absent["delivery_decision"], "not_evaluated");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["checker_id"], "rust.cargo_check",
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["verification_observation"], "candidate_absent_unverified_policy",
        "{next}"
    );
    assert_ne!(next["delivery_decision"], "allow");
}

#[test]
fn initialized_build_missing_tool_is_a_stable_preparation_task() {
    let fixture = Fixture::new();
    fixture.init();
    for _ in 0..2 {
        let (_, report) = fixture.check(&[]);
        assert_eq!(report["backlog_status"], "synced");
    }
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn queued_build_rejects_forged_fingerprint_and_stale_target_only_creates_preparation() {
    let fixture = Fixture::new();
    fixture.init();
    fs::copy(fixture.0.join("src/lib.rs"), fixture.0.join("src/bad.rs")).unwrap();
    fs::write(fixture.0.join("src/lib.rs"), "pub mod bad;\n").unwrap();
    let mut native: Value = serde_json::from_str(&fixture.error()).unwrap();
    native["message"]["spans"][0]["file_name"] = json!("src/bad.rs");
    let tool = fixture.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'\nexit 101",
        native
    ));
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(report["backlog_status"], "synced", "{report}");
    let queue = |mut value: Value, run: &str| {
        value["run_id"] = json!(run);
        value["backlog_status"] = json!("queued");
        value["backlog_sync"] = Value::Null;
        fs::write(
            fixture.0.join(format!(".codeguard/reports/{run}.json")),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    };
    let sync = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["work", "sync", fixture.0.to_str().unwrap(), "--format=json"])
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let mut forged = report.clone();
    forged["findings"][0]["finding_fingerprint"] = json!("0".repeat(64));
    queue(forged, "cargo-build-99-101");
    let result = sync();
    assert_eq!(result["failed_reports"], 1, "{result}");
    assert_eq!(result["new_findings"], 0);
    queue(report, "cargo-build-99-102");
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub mod bad;\npub fn changed() {}\n",
    )
    .unwrap();
    let result = sync();
    assert_eq!(result["new_findings"], 0);
    assert_eq!(result["historical_findings"], 1, "{result}");
    assert_eq!(result["new_blockers"], 1);
}

#[test]
fn cargo_proxy_preserves_selected_entrypoint_and_rejects_retargeting() {
    for retarget in [false, true] {
        let fixture = Fixture::new();
        let tools = Fixture::new();
        let mutation = if retarget {
            "ln -sf alternate-proxy \"$0\""
        } else {
            ""
        };
        let dispatcher = tools.tool(&format!(
            "case \"$0\" in */cargo-entry) ;; *) exit 91 ;; esac\n{mutation}\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'"
        ));
        let alternate = tools.0.join("alternate-proxy");
        fs::copy(&dispatcher, &alternate).unwrap();
        let entry = tools.0.join("cargo-entry");
        std::os::unix::fs::symlink(&dispatcher, &entry).unwrap();
        let (exit, report) = fixture.check(&["--cargo-tool", entry.to_str().unwrap()]);
        assert_eq!(exit, 3);
        assert_eq!(report["local_scan_complete"], !retarget, "{report}");
        assert_eq!(
            report["reason"],
            if retarget {
                "tool_changed_during_scan"
            } else {
                "native_observed_unverified"
            }
        );
    }
}

#[test]
fn lint_all_skips_historical_build_blocker_and_selects_current_clippy_task() {
    let fixture = Fixture::new();
    fixture.init();
    let (_, build) = fixture.check(&[]);
    assert_eq!(build["backlog_status"], "synced", "{build}");
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"message\":\"unneeded return\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "all",
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"], "clippy::needless_return",
        "{report}"
    );
    assert_eq!(
        report["next"]["repair_brief"]["checker_id"], "rust.cargo_clippy",
        "{report}"
    );
    let build_facts = fs::read_dir(fixture.0.join(".codeguard/findings"))
        .unwrap()
        .filter_map(|entry| {
            let bytes = fs::read(entry.ok()?.path().join("finding.json")).ok()?;
            serde_json::from_slice::<Value>(&bytes).ok()
        })
        .filter(|fact| fact["checker_id"] == "rust.cargo_check")
        .count();
    assert!(build_facts > 0, "历史构建阻塞不能被删除以绕过选择");
    if let Ok(path) = std::env::var("CODEGUARD_LINT_NEXT_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
