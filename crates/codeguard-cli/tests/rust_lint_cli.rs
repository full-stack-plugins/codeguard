#![cfg(unix)]
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
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-rust-lint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='lint-sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname=\"lint-sample\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), source).unwrap();
        Self(root)
    }
    fn invoke(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "rust"])
            .arg(&self.0)
            .args(args)
            .env("PATH", "")
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap()
    }
    fn report(&self, args: &[&str]) -> Value {
        let out = self.invoke(args);
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn tool(&self, body: &str) -> PathBuf {
        let tool = self.0.join("native-cargo");
        fs::write(&tool,format!("#!/bin/sh\n[ \"$1\" = clippy ] || exit 29\nprintf '%s\\n' \"$*\" > invoked\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn selected_native_cargo_runs_only_clippy_and_retains_local_diagnostic() {
    let project = Project::new("pub fn answer() -> i32 { return 42; }\n");
    let warning = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"message":"unneeded return","spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":25,"is_primary":true}]}});
    let tool = project.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        warning
    ));
    let r = project.report(&["--cargo-tool", tool.to_str().unwrap(), "--format=json"]);
    assert_eq!(r["report_type"], "rust_lint_feedback");
    assert_eq!(r["native_report"]["local_scan_complete"], true, "{r}");
    assert_eq!(
        r["native_report"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(r["syntax_candidates"]["reason"], "native_tool_selected");
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert_eq!(
        fs::read_to_string(project.0.join("invoked"))
            .unwrap()
            .trim(),
        "clippy --locked --offline --all-targets --message-format=json"
    );
}
#[test]
fn explicit_invalid_cargo_does_not_run_wasm_or_fallback_tool() {
    let p = Project::new("pub fn broken( {\n");
    let r = p.report(&["--cargo-tool", "/missing/cargo", "--format=json"]);
    assert_eq!(r["native_report"]["reason"], "cargo_tool_unavailable");
    assert_eq!(r["syntax_candidates"]["reason"], "native_tool_selected");
    assert_eq!(r["native_tool_requirement"], "required");
}
#[test]
fn invalid_arguments_are_rejected_before_native_execution() {
    let p = Project::new("pub fn f() {}\n");
    let tool = p.tool("exit 0");
    for extra in [
        vec!["--cargo-tool", tool.to_str().unwrap(), "--timeout", "0s"],
        vec![
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
        ],
        vec!["--format", "sarif"],
    ] {
        let out = p.invoke(&extra);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!p.0.join("invoked").exists());
    }
}
#[test]
fn missing_native_reports_wasm_build_boundary() {
    let p = Project::new("pub fn broken( {\n");
    let r = p.report(&["--format=json"]);
    assert_eq!(r["report_type"], "rust_lint_feedback");
    assert_eq!(r["tool_selection"]["source"], "not_found");
    assert_eq!(r["native_tool_requirement"], "required");
    if cfg!(feature = "wasm-precheck") {
        assert_eq!(r["preliminary_result"], "candidates_observed", "{r}");
        assert_eq!(
            r["syntax_candidates"]["observations"][0]["language"],
            "rust"
        );
    } else {
        assert_eq!(r["syntax_candidates"]["reason"], "wasm_feature_not_built");
    }
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn zero_wasm_candidates_recommend_cargo_without_quality_pass() {
    let p = Project::new("pub fn answer() -> i32 { 42 }\n");
    let r = p.report(&["--format=json"]);
    assert_eq!(r["preliminary_result"], "no_candidates_observed", "{r}");
    assert_eq!(r["native_tool_requirement"], "recommended");
    assert_eq!(r["exit_code"], 3);
}

#[test]
fn empty_source_scope_does_not_start_selected_cargo() {
    let p = Project::new("pub fn f() {}\n");
    fs::remove_file(p.0.join("src/lib.rs")).unwrap();
    let tool = p.tool("exit 0");
    let r = p.report(&["--cargo-tool", tool.to_str().unwrap(), "--format=json"]);
    assert!(r["native_report"].is_null());
    assert_eq!(
        r["syntax_candidates"]["reason"],
        "rust_source_scope_unavailable"
    );
    assert!(!p.0.join("invoked").exists());
}
#[test]
fn repeated_lint_reuses_one_task_and_original_verification() {
    let p = Project::new("pub fn f() {}\n");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", p.0.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let warning = json!({"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::needless_return"},"spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"is_primary":true}]}});
    let tool = p.tool(&format!(
        "printf '%s\\n' '{}' '{{\"reason\":\"build-finished\",\"success\":true}}'",
        warning
    ));
    let first = p.report(&["--cargo-tool", tool.to_str().unwrap(), "--format=json"]);
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    let second = p.report(&["--cargo-tool", tool.to_str().unwrap(), "--format=json"]);
    assert_eq!(second["next"]["repair_brief"]["task_id"], id);
    assert_eq!(second["native_report"]["backlog_sync"]["new_findings"], 0);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--cargo-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verify["observation"], "still_present");
}
#[test]
#[ignore = "requires explicitly selected installed Cargo with Clippy; no installation"]
fn real_clippy_finding_fix_and_original_task_verification() {
    let tool = std::env::var("CODEGUARD_TEST_CARGO").expect("select installed Cargo");
    let p = Project::new("pub fn answer() -> i32 { return 42; }\n");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", p.0.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "rust",
                p.0.to_str().unwrap(),
                "--cargo-tool",
                &tool,
                "--format=json",
            ])
            .output()
            .unwrap()
    };
    let out = invoke();
    assert_eq!(out.status.code(), Some(3));
    let first: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        first["native_report"]["local_scan_complete"], true,
        "{first}"
    );
    assert!(
        first["native_report"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["rule_id"] == "clippy::needless_return")
    );
    let id = first["next"]["repair_brief"]["task_id"].as_str().unwrap();
    fs::write(p.0.join("src/lib.rs"), "pub fn answer() -> i32 { 42 }\n").unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--cargo-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        result["observation"], "candidate_absent_unverified_policy",
        "{result}"
    );
    assert_eq!(result["native_scan"]["local_scan_complete"], true);
    assert_eq!(
        result["native_scan"]["suppression_probe"]["forced_scan"]["local_scan_complete"],
        true
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Ok(dir) = std::env::var("CODEGUARD_RUST_LINT_EVIDENCE_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(PathBuf::from(&dir).join("native.json"), &out.stdout).unwrap();
        fs::write(PathBuf::from(&dir).join("verify.json"), &verify.stdout).unwrap();
    }
}
