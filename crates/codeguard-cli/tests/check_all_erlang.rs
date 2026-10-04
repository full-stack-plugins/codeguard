#![cfg(unix)]

use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-check-erlang-{label}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("sample.erl"), "-module(sample).\nf() -> ok\n").unwrap();
        Self(root)
    }
    fn fake(&self, version: &str, preprocessing: bool, extra: &str) -> PathBuf {
        fs::create_dir(self.0.join("bin")).unwrap();
        let tool = self.0.join("bin/erl");
        fs::write(&tool, format!(
            "#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' '{version}';; *) /bin/cat >/dev/null; {extra}\nprintf '%s\\n' '{{\"schema_version\":\"0.1.0\",\"forms\":1,\"preprocessing\":{preprocessing},\"diagnostics_truncated\":false,\"diagnostics\":[{{\"line\":2,\"column\":8,\"rule_id\":\"erlang.syntax.error\"}}]}}';; esac\n"
        )).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn output(&self, selection: &str, extra: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", selection])
            .arg(&self.0)
            .args(["--timeout", "60s"])
            .args(extra)
            .env("PATH", self.0.join("bin"))
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn check(&self, extra: &[&str]) -> Value {
        let out = self.output("all", extra);
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn native_path_diagnostic_is_retained_with_original_tool_guidance() {
    let p = Project::new("path");
    let tool = p.fake("OTP 28", false, "");
    let report = p.check(&["--format=json"]);
    assert_eq!(report["schema_version"], "0.36.0");
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["tool_selection"]["source"], "path");
    assert_eq!(
        native["tool_selection"]["executable"],
        tool.to_str().unwrap()
    );
    assert_eq!(
        native["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
    assert_eq!(native["files"][0]["findings"][0]["line"], 2);
    assert_eq!(native["files"][0]["recheck_argv"][4], "--erl-tool");
    assert_eq!(
        native["files"][0]["recheck_argv"][5],
        tool.to_str().unwrap()
    );
    assert_eq!(native["local_forms_complete"], true);
    assert_eq!(native["coverage_proven"], false);
    assert_eq!(report["execution_tasks"][0]["id"], "erlang.lint");
    assert_eq!(report["execution_budget"]["native_task_count"], 1);
    let candidate = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["language"] == "erlang" && v["category"] == "lint")
        .unwrap();
    assert_eq!(candidate["status"], "native_incomplete");
    assert_eq!(candidate["checker_id"], "erlang.otp.forms");
    assert_eq!(report["delivery_decision"], "incomplete");
    #[cfg(feature = "wasm-precheck")]
    {
        assert_eq!(report["syntax_candidates"]["native_preferred_count"], 1);
        assert!(
            report["syntax_candidates"]["observations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let human = p.output("all", &["--format=human"]);
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("Erlang"));
    assert!(text.contains("sample.erl:2:8"));
}

#[test]
fn absent_tool_and_selected_tool_failure_remain_distinct() {
    let p = Project::new("absent");
    let missing = p.check(&["--format=json"]);
    assert_eq!(
        missing["native_results"]["erlang_lint"]["tool_selection"]["source"],
        "not_found"
    );
    assert_eq!(
        missing["native_results"]["erlang_lint"]["files"][0]["native"]["status"],
        "not_run"
    );
    p.fake("OTP 28", false, "");
    let bad = p.0.join("missing-tool");
    let report = p.check(&["--erl-tool", bad.to_str().unwrap(), "--format=json"]);
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["tool_selection"]["source"], "explicit");
    assert_eq!(native["files"][0]["native"]["status"], "incomplete");
    assert_eq!(native["files"][0]["recheck_argv"], Value::Null);
    assert_eq!(native["local_forms_complete"], false);
}

#[test]
fn preprocessing_never_becomes_a_source_finding_or_native_coverage() {
    let p = Project::new("preprocessing");
    p.fake("OTP 28", true, "");
    let report = p.check(&["--format=json"]);
    let file = &report["native_results"]["erlang_lint"]["files"][0];
    assert_eq!(file["native"]["reason"], "erlang_preprocessing_unresolved");
    assert!(file["findings"].as_array().unwrap().is_empty());
    assert_eq!(
        report["native_results"]["erlang_lint"]["local_forms_complete"],
        false
    );
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
}

#[test]
fn source_changes_discard_stale_positions_and_recheck_guidance() {
    let p = Project::new("mutation");
    let extra = format!(
        "printf '%s\\n' '-module(sample).' 'f() -> ok.' > '{}'",
        p.0.join("sample.erl").display()
    );
    p.fake("OTP 28", false, &extra);
    let report = p.check(&["--format=json"]);
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["local_forms_complete"], false);
    assert!(
        native["files"][0]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(native["files"][0]["recheck_argv"], Value::Null);
    assert!(
        native["files"][0]["native"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn changed_project_scope_preserves_still_current_file_diagnostics() {
    let p = Project::new("scope-change");
    let extra = format!(
        "printf '%s\\n' '-module(added).' 'g() -> ok.' > '{}'",
        p.0.join("added.erl").display()
    );
    p.fake("OTP 28", false, &extra);
    let report = p.check(&["--format=json"]);
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["scope_stable"], false);
    assert_eq!(native["local_forms_complete"], false);
    assert_eq!(native["files"][0]["current"], true);
    assert_eq!(native["files"][0]["findings"][0]["line"], 2);
    assert!(!native["files"][0]["recheck_argv"].is_null());
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "project_scope_changed_during_check")
    );
}

#[test]
fn sarif_keeps_native_findings_without_claiming_project_success() {
    let p = Project::new("sarif");
    p.fake("OTP 28", false, "");
    let report = p.check(&["--format=sarif"]);
    assert_eq!(report["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(
        report["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "incomplete"
    );
}

#[test]
fn original_tool_recheck_works_outside_the_selected_project() {
    let p = Project::new("recheck-cwd");
    let tool = p.fake("OTP 28", false, "");
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--erl-tool", tool.to_str().unwrap(), "--format=json"])
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .current_dir("/")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let argv = report["native_results"]["erlang_lint"]["files"][0]["recheck_argv"]
        .as_array()
        .unwrap();
    assert_eq!(argv[3], p.0.join("sample.erl").to_str().unwrap());
    let recheck = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(argv.iter().skip(1).map(|v| v.as_str().unwrap()))
        .current_dir("/")
        .output()
        .unwrap();
    assert_eq!(recheck.status.code(), Some(3));
    let checked: Value = serde_json::from_slice(&recheck.stdout).unwrap();
    assert_eq!(checked["native"]["status"], "diagnostics_observed");
    assert_eq!(checked["native"]["diagnostics"][0]["column"], 8);
}

#[test]
fn java_selection_and_invalid_arguments_do_not_start_erlang() {
    let p = Project::new("selection");
    let marker = p.0.join("executed");
    let tool = p.fake(
        "OTP 28",
        false,
        &format!("/usr/bin/touch '{}'", marker.display()),
    );
    let out = p.output("java", &["--format=json"]);
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["native_results"]["erlang_lint"], Value::Null);
    assert!(!marker.exists());
    let out = p.output(
        "java",
        &["--erl-tool", tool.to_str().unwrap(), "--format=json"],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(!marker.exists());
    let out = p.output(
        "all",
        &[
            "--erl-tool",
            tool.to_str().unwrap(),
            "--erl-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(!marker.exists());
}

#[test]
fn file_budget_retains_unobserved_scope_instead_of_claiming_completion() {
    let p = Project::new("budget");
    for i in 0..64 {
        fs::write(
            p.0.join(format!("file_{i:02}.erl")),
            "-module(sample).\nf() -> ok.\n",
        )
        .unwrap();
    }
    let report = p.check(&["--format=json"]);
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["source_file_count"], 65);
    assert_eq!(native["files"].as_array().unwrap().len(), 64);
    assert_eq!(native["unobserved_count"], 1);
    assert_eq!(native["local_forms_complete"], false);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn changed_tool_and_timeout_cannot_supply_stale_findings() {
    let p = Project::new("tool-change");
    p.fake("OTP 28", false, "printf '# changed\\n' >> \"$0\"");
    let report = p.check(&["--format=json"]);
    let native = &report["native_results"]["erlang_lint"];
    assert_eq!(native["local_forms_complete"], false);
    assert!(
        native["files"][0]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(native["files"][0]["recheck_argv"], Value::Null);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--timeout", "1ms", "--format=json"])
        .env("PATH", p.0.join("bin"))
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let timed: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(timed["delivery_decision"], "incomplete");
    assert_ne!(
        timed["native_results"]["erlang_lint"]["local_forms_complete"],
        true
    );
}

#[test]
fn aggregate_cancellation_preserves_exit_code_and_reaps_descendants() {
    let p = Project::new("cancel");
    let ready = p.0.join("ready");
    let escaped = p.0.join("escaped");
    p.fake(
        "OTP 28",
        false,
        &format!(
            "/usr/bin/touch '{}'; /bin/sleep 2; /usr/bin/touch '{}'",
            ready.display(),
            escaped.display()
        ),
    );
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--format=json", "--timeout", "20s"])
        .env("PATH", p.0.join("bin"))
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(ready.exists(), "native process did not start");
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["reason"], "request_cancelled");
    assert_eq!(report["syntax_candidates"]["status"], "not_run");
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["reason"], "request_cancelled");
    #[cfg(not(feature = "wasm-precheck"))]
    assert_eq!(
        report["syntax_candidates"]["reason"],
        "binary_without_wasm_precheck"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert!(!escaped.exists(), "native descendant escaped cancellation");
}

#[test]
#[ignore = "requires existing OTP 28 via CODEGUARD_ERL_BIN"]
fn real_otp_reports_missing_period_and_clean_sibling() {
    let p = Project::new("actual");
    fs::write(p.0.join("good.erl"), "-module(good).\nf() -> ok.\n").unwrap();
    let tool = std::env::var("CODEGUARD_ERL_BIN").unwrap();
    let report = p.check(&["--erl-tool", &tool, "--format=json"]);
    let files = report["native_results"]["erlang_lint"]["files"]
        .as_array()
        .unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["native"]["status"], "completed");
    assert_eq!(files[1]["native"]["status"], "diagnostics_observed");
    assert_eq!(files[1]["findings"][0]["column"], 8);
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 2);
}
