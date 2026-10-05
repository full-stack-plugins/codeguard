#![cfg(unix)]
use serde_json::Value;
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
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-ruby-lint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("app.rb"), source).unwrap();
        Self(p)
    }
    fn init(&self) {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", self.0.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
    }
    fn tool(&self, body: &str) -> PathBuf {
        let p = self.0.join("ruby-native");
        fs::write(&p,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruby 2.6.10p210 (fixture) [test]\\n'; exit 0; fi\n/bin/cat > /dev/null\nprintf '%s\\n' \"$*\" > '{}/invoked'\n{body}\n",self.0.display())).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn lint(&self, args: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "ruby"])
            .arg(self.0.join("app.rb"))
            .args(["--format=json"])
            .args(args)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn native_ruby_syntax_is_visible_without_fabricating_columns_or_style_lint() {
    let p = Project::new("def f(\n");
    let tool = p.tool("printf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1");
    let r = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(r["report_type"], "ruby_lint_feedback");
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert_eq!(r["native"]["diagnostics"][0]["line"], 1);
    assert!(r["native"]["diagnostics"][0].get("column").is_none());
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert_eq!(r["syntax_precheck"], Value::Null);
    assert_eq!(
        fs::read_to_string(p.0.join("invoked")).unwrap().trim(),
        "--disable=gems -EUTF-8:UTF-8 -W0 -c -"
    );
}
#[test]
fn native_first_task_repeats_and_original_tool_rechecks() {
    let p = Project::new("def f(\n");
    p.init();
    let tool = p.tool("printf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1");
    let first = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    let id = first["task_id"].as_str().unwrap();
    let again = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(again["task_id"], id);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["schema_version"], "0.15.0", "{next}");
    assert_eq!(next["repair_brief"]["native_column_unit"], "unavailable");
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"][0]["line"],
        1
    );
    assert!(
        next["repair_brief"]["native_diagnostic_positions"][0]
            .get("column")
            .is_none()
    );
    assert_eq!(next["repair_brief"]["recheck_argv"][7], "--ruby-tool");
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["observation"], "still_blocked", "{r}");
    assert_eq!(r["native_scan"]["schema_version"], "0.10.0");
    assert_eq!(r["event_persisted"], true);
}
#[test]
fn explicit_missing_ruby_never_uses_wasm_as_success() {
    let p = Project::new("def f(\n");
    let r = p.lint(&["--ruby-tool", "/missing/ruby"]);
    assert_eq!(r["native"]["status"], "incomplete");
    assert!(r["syntax_precheck"].is_null());
    assert_eq!(r["setup"]["native_confirmation_required"], true);
}
#[test]
fn missing_native_observation_has_honest_build_boundary() {
    let p = Project::new("def f(\n");
    let r = p.lint(&[]);
    if cfg!(feature = "wasm-precheck") {
        assert!(
            !r["syntax_precheck"]["recoveries"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{r}"
        );
    } else {
        assert_eq!(r["syntax_precheck"]["reason"], "wasm_feature_not_built");
    }
    assert_eq!(r["setup"]["native_confirmation_required"], true);
}
#[test]
#[ignore = "requires explicitly selected existing Ruby 2.6.10p210; no installation"]
fn real_ruby_repair_keeps_task_open_until_authorized_closure() {
    let tool = std::env::var("CODEGUARD_TEST_RUBY").expect("select installed Ruby");
    let p = Project::new("def f(\n");
    p.init();
    let first = p.lint(&["--ruby-tool", &tool]);
    let id = first["task_id"].as_str().unwrap();
    fs::write(p.0.join("app.rb"), "def f()\n  1\nend\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--ruby-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native_scan"]["native"]["status"], "completed", "{r}");
    assert_eq!(r["observation"], "candidate_absent_unverified_policy");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Ok(dir) = std::env::var("CODEGUARD_RUBY_EVIDENCE_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(&dir).join("native.json"),
            serde_json::to_vec(&first).unwrap(),
        )
        .unwrap();
        fs::write(PathBuf::from(&dir).join("verify.json"), out.stdout).unwrap();
    }
}

#[test]
fn ruby_invalid_arguments_do_not_start_selected_tool() {
    let p = Project::new("puts 1\n");
    let tool = p.tool("printf 'Syntax OK\\n'; exit 0");
    for extra in [
        vec!["--ruby-tool", "relative/ruby"],
        vec!["--timeout", "0"],
        vec!["--format", "sarif"],
        vec![
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
        ],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "ruby"])
            .arg(p.0.join("app.rb"))
            .args(extra)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!p.0.join("invoked").exists());
    }
}

#[test]
fn ruby_newer_version_is_incomplete_and_does_not_trigger_fallback() {
    let p = Project::new("puts 1\n");
    let tool = p.tool("printf 'Syntax OK\\n'; exit 0");
    let source = fs::read_to_string(&tool)
        .unwrap()
        .replace("2.6.10p210", "3.4.0");
    fs::write(&tool, source).unwrap();
    let r = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(r["native"]["reason"], "ruby_syntax_version_unverified");
    assert!(r["syntax_precheck"].is_null());
    assert!(!p.0.join("invoked").exists());
}

#[test]
fn ruby_source_change_discards_observed_diagnostics() {
    let p = Project::new("def f(\n");
    let tool = p.tool(&format!("printf 'puts 1\\n' > '{}/app.rb'\nprintf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1", p.0.display()));
    let r = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(r["native"]["reason"], "ruby_source_changed_during_check");
    assert_eq!(r["input_stable"], false);
    assert!(r["native"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(r["syntax_precheck"].is_null());
}

#[test]
fn ruby_completed_syntax_still_leaves_project_policy_unknown() {
    let p = Project::new("puts 1\n");
    let tool = p.tool("printf 'Syntax OK\\n'; exit 0");
    let r = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(r["native"]["status"], "completed");
    assert_eq!(r["setup"]["full_project_checks_required"], true);
    assert_eq!(r["project_version_compatibility"], "unverified");
    assert_eq!(r["coverage_proven"], false);
}

#[test]
fn ruby_zero_wasm_candidates_recommend_native_without_claiming_pass() {
    let p = Project::new("puts 1\n");
    let r = p.lint(&[]);
    if cfg!(feature = "wasm-precheck") {
        assert!(
            r["syntax_precheck"]["recoveries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(r["setup"]["native_confirmation_required"], false);
    } else {
        assert_eq!(r["setup"]["native_confirmation_required"], true);
    }
    assert_eq!(r["command_status"], "incomplete");
    assert_eq!(r["delivery_decision"], "not_evaluated");
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn ruby_wasm_candidate_and_native_discovery_share_task_identity() {
    let p = Project::new("def f(\n");
    p.init();
    let fallback = p.lint(&[]);
    let id = fallback["task_id"]
        .as_str()
        .expect("candidate must create task");
    let tool = p.tool("printf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1");
    let native = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    assert_eq!(native["task_id"], id);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["observation"], "still_blocked", "{report}");
    assert_eq!(report["event_persisted"], true);
}

#[test]
fn ruby_task_rejects_wrong_or_relative_tools_before_invocation() {
    let p = Project::new("def f(\n");
    p.init();
    let tool = p.tool("printf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1");
    let first = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    let id = first["task_id"].as_str().unwrap();
    fs::remove_file(p.0.join("invoked")).unwrap();
    for extra in [
        vec!["--ruby-tool", "relative/ruby"],
        vec!["--swift-tool", tool.to_str().unwrap()],
        vec![
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
        ],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify", id, p.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(!p.0.join("invoked").exists());
    }
}

#[test]
fn ruby_failed_recheck_records_attempt_and_clears_old_repair_positions() {
    let p = Project::new("def f(\n");
    p.init();
    let tool = p.tool("printf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2; exit 1");
    let first = p.lint(&["--ruby-tool", tool.to_str().unwrap()]);
    let id = first["task_id"].as_str().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--ruby-tool",
            "/missing/ruby",
            "--format=json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["observation"], "incomplete");
    assert_eq!(report["event_persisted"], true);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["native_confirmation_status"], "incomplete",
        "{next}"
    );
    assert!(
        next["repair_brief"]["native_diagnostic_positions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_ne!(next["repair_brief"]["disposition"], "actionable");
}
