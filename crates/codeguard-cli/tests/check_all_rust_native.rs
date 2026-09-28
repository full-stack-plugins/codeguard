#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "codeguard-rust-native-{}-{nonce}-{}",
            std::process::id(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='rust-sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n").unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname=\"rust-sample\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        Self(root)
    }

    fn tool(&self, body: &str) -> PathBuf {
        let path = self.0.join("cargo-native");
        fs::write(&path, format!("#!/bin/sh\ncase \"$1\" in rustdoc) printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'; exit 0 ;; esac\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn check(&self, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_all_schedules_native_cargo_type_check_and_keeps_compiler_error_separate() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { \"wrong\" }\n",
    )
    .unwrap();
    let root = fixture.0.canonicalize().unwrap();
    let error = serde_json::json!({"reason":"compiler-message","package_id":"path+file:///example#rust-sample@0.1.0","manifest_path":root.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":root.join("src/lib.rs")},"message":{"level":"error","code":{"code":"E0308"},"spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":25,"byte_start":24,"byte_end":31,"is_primary":true}]}});
    let tool = fixture.tool(&format!(
        "if [ \"$1\" = check ]; then printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":false}}'; exit 101; fi\nprintf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        error
    ));
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_build"]["local_scan_complete"], true,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["rust_build"]["findings"][0]["rule_id"],
        "E0308"
    );
    assert_eq!(
        report["native_results"]["rust_build"]["test_execution"],
        false
    );
    assert_eq!(
        report["native_results"]["rust_build"]["build_level"],
        "type_check"
    );
    assert_eq!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "rust" && row["category"] == "build")
            .unwrap()["checker_id"],
        "rust.cargo_check"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn cargo_clippy_native_warning_is_visible_without_granting_delivery() {
    let fixture = Fixture::new();
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"message\":\"unneeded return\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap(), "--jobs", "2"]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        true
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["path"],
        "src/lib.rs"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(report["execution_budget"]["native_task_count"], 4);
    assert_eq!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "rust" && row["category"] == "lint")
            .unwrap()["status"],
        "observed_unverified"
    );
}

#[test]
fn missing_cargo_or_invalid_native_report_stays_incomplete() {
    let fixture = Fixture::new();
    let (_, missing) = fixture.check(&[]);
    assert_eq!(
        missing["native_results"]["rust_lint"]["reason"],
        "cargo_tool_not_selected"
    );
    assert_eq!(
        missing["native_results"]["rust_lint"]["configuration"],
        "unknown"
    );
    let tool = fixture
        .tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}' 'bad-json'");
    let (_, malformed) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(
        malformed["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(
        malformed["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "rust" && row["category"] == "lint")
            .unwrap()["status"],
        "native_incomplete"
    );
}

#[test]
fn clippy_config_is_observed_without_equating_manifest_with_checker_configuration() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("clippy.toml"), "msrv = \"1.85.0\"\n").unwrap();
    let (_, report) = fixture.check(&[]);
    assert_eq!(
        report["native_results"]["rust_lint"]["configuration"],
        "configured"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["configuration_ref"],
        "clippy.toml"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "cargo_tool_not_selected"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    let visible = String::from_utf8(human.stdout).unwrap();
    assert!(visible.contains("Rust/Cargo Clippy 配置: configured"));
    assert!(visible.contains("cargo_tool_not_selected"));
    assert!(visible.contains("复检: cargo clippy --offline --all-targets"));
}

#[test]
fn valid_clippy_finding_survives_a_later_malformed_record() {
    let fixture = Fixture::new();
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' 'bad-json'");
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "native_report_malformed"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn compilation_error_is_not_mislabeled_as_a_clippy_finding() {
    let fixture = Fixture::new();
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"code\":{\"code\":\"E0308\"},\"spans\":[]}}' '{\"reason\":\"build-finished\",\"success\":false}'; exit 101");
    let (_, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"],
        serde_json::json!([])
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "non_clippy_compilation_error"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn initialized_rust_finding_becomes_one_stable_repair_task() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(
        first["native_results"]["rust_lint"]["backlog_status"],
        "synced_partial"
    );
    assert_eq!(first["next"]["repair_brief"]["kind"], "finding");
    assert_eq!(
        first["next"]["repair_brief"]["native_rule_id"],
        "clippy::needless_return"
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    assert!(
        fixture
            .0
            .join(".codeguard/tasks")
            .join(format!("{id}.md"))
            .is_file()
    );
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains(&format!("下一步任务 \"{id}\""))
    );
    let (_, second) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        2
    );
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(observation["observation"], "still_present");
    assert_eq!(observation["event_persisted"], true);
    assert_eq!(
        observation["native_scan"]["report_type"],
        "rust_clippy_local_observation"
    );
    assert_eq!(observation["delivery_decision"], "not_evaluated");
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "still_present"
    );
}

#[test]
fn rust_recheck_without_finding_requires_rule_coverage_review() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let clean = fixture.tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            clean.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        observation["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(observation["event_persisted"], true);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(next["repair_brief"]["disposition"], "verification_required");
}

#[test]
fn broken_force_warn_probe_cannot_make_a_rust_task_look_fixed() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (_, first) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let broken = fixture.tool("case \"$*\" in *--force-warn*) printf '%s\\n' bad-json;; *) printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}';; esac");
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            broken.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        observation["native_scan"]["suppression_probe"]["forced_scan"]["local_scan_complete"],
        false
    );
    assert_eq!(observation["observation"], "incomplete");
    assert_eq!(observation["delivery_decision"], "not_evaluated");
}

#[test]
fn initialized_missing_cargo_becomes_an_environment_task() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let (_, first) = fixture.check(&[]);
    assert_eq!(
        first["native_results"]["rust_lint"]["backlog_status"],
        "synced_partial"
    );
    assert_eq!(first["next"]["repair_brief"]["kind"], "blocker");
    assert_eq!(
        first["next"]["repair_brief"]["reason_code"],
        "cargo_tool_not_selected"
    );
    assert_eq!(
        first["next"]["repair_brief"]["checker_id"],
        "rust.cargo_clippy"
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let (_, second) = fixture.check(&[]);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        4
    );
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"build-finished\",\"success\":true}'");
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        observation["observation"],
        "environment_restored_unverified_policy"
    );
    assert_eq!(observation["event_persisted"], true);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["repair_brief"]["checker_id"], "rust.cargo_rustdoc");
    assert_eq!(
        next["repair_brief"]["reason_code"],
        "cargo_tool_not_selected"
    );
    assert_ne!(next["repair_brief"]["task_id"], id);
}

#[test]
fn project_writable_decision_cannot_hide_a_clippy_finding() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::write(
        fixture.0.join(".codeguard/decisions/local.json"),
        r#"{"kind":"false_positive","native_rule_id":"clippy::needless_return","approved":true}"#,
    )
    .unwrap();
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (exit, report) = fixture.check(&["--cargo-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["backlog_status"],
        "synced_partial"
    );
    assert_eq!(report["next"]["repair_brief"]["kind"], "finding");
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn mixed_project_preserves_rust_finding_when_python_checker_is_unavailable() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("app.py"), "import os\n").unwrap();
    fs::write(fixture.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = fixture.tool("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"message\":\"unneeded return\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' '{\"reason\":\"build-finished\",\"success\":true}'");
    let (_, report) = fixture.check(&[
        "--cargo-tool",
        tool.to_str().unwrap(),
        "--ruff-tool",
        "/nonexistent/ruff",
        "--jobs",
        "2",
    ]);
    assert_eq!(report["execution_budget"]["native_task_count"], 5);
    assert_eq!(report["execution_budget"]["started_native_task_count"], 5);
    assert_eq!(report["execution_tasks"][0]["id"], "python.lint");
    assert_eq!(report["execution_tasks"][1]["id"], "rust.cve");
    assert_eq!(report["execution_tasks"][2]["id"], "rust.build");
    assert_eq!(report["execution_tasks"][3]["id"], "rust.comments");
    assert_eq!(report["execution_tasks"][4]["id"], "rust.lint");
    assert_eq!(report["execution_tasks"][0]["status"], "native_incomplete");
    assert_eq!(
        report["execution_tasks"][4]["status"],
        "native_observed_unverified"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
#[ignore = "requires an installed native Cargo Clippy toolchain; set CODEGUARD_CARGO_BIN"]
fn real_cargo_clippy_observes_a_rust_project() {
    let tool = std::env::var("CODEGUARD_CARGO_BIN").expect("原生 Cargo 绝对路径");
    let fixture = Fixture::new();
    let (exit, report) = fixture.check(&["--cargo-tool", &tool, "--timeout", "60s"]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"], "native_observed_unverified",
        "{report}"
    );
    assert_eq!(
        report["native_results"]["rust_lint"]["local_scan_complete"],
        true
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    let documentation = &report["native_results"]["rust_comments"];
    assert_eq!(documentation["local_scan_complete"], true, "{report}");
    assert_eq!(documentation["coverage_proven"], false);
    assert!(
        documentation["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["rule_id"] == "missing_docs" }),
        "{report}"
    );
    assert_eq!(report["execution_budget"]["native_task_count"], 4);
}

#[test]
#[ignore = "requires an installed native Cargo Clippy toolchain; set CODEGUARD_CARGO_BIN"]
fn real_cargo_clippy_rechecks_a_persistent_task() {
    let tool = std::env::var("CODEGUARD_CARGO_BIN").expect("原生 Cargo 绝对路径");
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { return 42; }\n",
    )
    .unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let (_, first) = fixture.check(&["--cargo-tool", &tool, "--timeout", "60s"]);
    assert_eq!(
        first["native_results"]["rust_lint"]["local_scan_complete"], true,
        "{first}"
    );
    let id = first["native_results"]["rust_lint"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["rule_id"] == "clippy::needless_return")
        .and_then(|finding| finding["finding_id"].as_str())
        .expect("真实 Clippy 应报告 needless_return");
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            &tool,
            "--timeout",
            "60s",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let observation: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(observation["observation"], "still_present", "{observation}");
    assert_eq!(observation["event_persisted"], true);
    fs::write(
        fixture.0.join("src/lib.rs"),
        "#[allow(clippy::needless_return)]\npub fn answer() -> i32 { return 42; }\n",
    )
    .unwrap();
    let suppressed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            fixture.0.to_str().unwrap(),
            "--cargo-tool",
            &tool,
            "--timeout",
            "60s",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(suppressed.status.code(), Some(3));
    let suppressed: Value = serde_json::from_slice(&suppressed.stdout).unwrap();
    assert_eq!(
        suppressed["observation"], "suppression_requires_review",
        "{suppressed}"
    );
    assert_eq!(suppressed["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["verification_observation"],
        "suppression_requires_review"
    );
    assert_eq!(next["repair_brief"]["disposition"], "needs_decision");
    fs::write(
        fixture.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    let changed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let changed: Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(
        changed["repair_brief"]["disposition"],
        "verification_required"
    );
}
