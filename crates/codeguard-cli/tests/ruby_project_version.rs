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
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-ruby-version-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.rb"), "def f(\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("ruby");
        fs::write(&tool,format!("#!/bin/sh\nprintf invoked > \"$0.called\"\nif [ \"$1\" = --version ]; then printf 'ruby {version} (fixture) [test]\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2\nexit 1\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, language: &str, tool: Option<&PathBuf>) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command
            .args(["check", language])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", &self.0);
        if let Some(tool) = tool {
            command.arg("--ruby-tool").arg(tool);
        }
        let out = command.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn init(&self) {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&self.0)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
    }
}
#[test]
fn declared_other_ruby_version_blocks_native_without_source_diagnostics() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "3.4.0\n").unwrap();
    let r = p.check("ruby", Some(&tool));
    let n = &r["native_results"]["ruby_lint"]["files"][0]["native"];
    assert_eq!(n["status"], "incomplete");
    assert_eq!(n["reason"], "ruby_project_version_mismatch");
    assert_eq!(n["diagnostics"], serde_json::json!([]));
    assert!(!p.0.join("ruby.called").exists());
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(
        r["syntax_candidates"]["observations"],
        serde_json::json!([])
    );
}
#[test]
fn matching_pin_is_observed_without_treating_it_as_full_project_approval() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "ruby-2.6.10\n").unwrap();
    let r = p.check("ruby", Some(&tool));
    assert_eq!(
        r["native_results"]["ruby_lint"]["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
    assert_ne!(r["delivery_decision"], "allow");
}
#[test]
fn unsupported_or_unsafe_version_declarations_do_not_start_old_ruby() {
    for text in ["system\n", "2.6\n", "2.6.10\n3.4.0\n", ""] {
        let p = Project::new();
        let tool = p.tool("2.6.10p210");
        fs::write(p.0.join(".ruby-version"), text).unwrap();
        let r = p.check("ruby", Some(&tool));
        assert_eq!(
            r["native_results"]["ruby_lint"]["files"][0]["native"]["reason"],
            "ruby_project_version_unresolved"
        );
        assert!(!p.0.join("ruby.called").exists());
    }
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join("version"), "2.6.10\n").unwrap();
    std::os::unix::fs::symlink(p.0.join("version"), p.0.join(".ruby-version")).unwrap();
    let r = p.check("ruby", Some(&tool));
    assert_eq!(
        r["native_results"]["ruby_lint"]["files"][0]["native"]["reason"],
        "ruby_project_version_unreadable"
    );
    assert!(!p.0.join("ruby.called").exists());
}
#[test]
fn declaration_changes_during_parse_withdraw_native_positions() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "2.6.10\n").unwrap();
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace(
            "/bin/cat >/dev/null",
            &format!(
                "/bin/cat >/dev/null\nprintf '3.4.0\\n' > '{}'",
                p.0.join(".ruby-version").display()
            ),
        ),
    )
    .unwrap();
    let r = p.check("ruby", Some(&tool));
    let n = &r["native_results"]["ruby_lint"]["files"][0]["native"];
    assert_eq!(n["reason"], "ruby_project_version_changed_during_check");
    assert_eq!(n["diagnostics"], serde_json::json!([]));
}

#[test]
fn changed_declaration_invalidates_saved_positions_and_recheck_reports_environment() {
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "2.6.10\n").unwrap();
    let r = p.check("ruby", Some(&tool));
    let id = r["native_results"]["ruby_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    fs::write(p.0.join(".ruby-version"), "3.4.0\n").unwrap();
    fs::remove_file(p.0.join("ruby.called")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", p.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"],
        serde_json::json!([])
    );
    assert_eq!(next["repair_brief"]["native_confirmation_status"], "stale");
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
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["native_scan"]["native"]["reason"],
        "ruby_project_version_mismatch"
    );
    assert!(!p.0.join("ruby.called").exists());
}
#[test]
fn standalone_ruby_lint_and_edit_hook_apply_the_project_pin() {
    use std::io::Write;
    use std::process::Stdio;
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "3.4.0\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "ruby",
            p.0.join("app.rb").to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native"]["reason"], "ruby_project_version_mismatch");
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--ruby-tool",
            tool.to_str().unwrap(),
            "--timeout",
            "30s",
            "--format=json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.rb"],"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        r["local_feedback"]["ruby_lint"]["files"][0]["native"]["reason"],
        "ruby_project_version_mismatch"
    );
    assert!(!p.0.join("ruby.called").exists());
}

#[test]
fn nearest_module_pin_is_selected_without_overwriting_sibling_environment_failure() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "3.4.0\n").unwrap();
    fs::create_dir(p.0.join("module")).unwrap();
    fs::write(p.0.join("module/.ruby-version"), "2.6.10\n").unwrap();
    fs::write(p.0.join("module/nested.rb"), "def f(\n").unwrap();
    let r = p.check("ruby", Some(&tool));
    let files = r["native_results"]["ruby_lint"]["files"]
        .as_array()
        .unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(
        files[0]["native"]["reason"],
        "ruby_project_version_mismatch"
    );
    assert_eq!(files[1]["native"]["status"], "diagnostics_observed");
    assert_eq!(
        r["native_results"]["ruby_lint"]["local_parse_complete"],
        false
    );
}
#[test]
fn absent_version_file_added_by_parser_invalidates_current_observation() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace(
            "/bin/cat >/dev/null",
            &format!(
                "/bin/cat >/dev/null\nprintf '2.6.10\\n' > '{}'",
                p.0.join(".ruby-version").display()
            ),
        ),
    )
    .unwrap();
    let r = p.check("ruby", Some(&tool));
    assert_eq!(
        r["native_results"]["ruby_lint"]["files"][0]["native"]["reason"],
        "ruby_project_version_changed_during_check"
    );
    assert_eq!(
        r["native_results"]["ruby_lint"]["files"][0]["native"]["diagnostics"],
        serde_json::json!([])
    );
}

#[test]
fn nested_gemfile_does_not_hide_initialized_workspace_version() {
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join(".ruby-version"), "3.4.0\n").unwrap();
    fs::create_dir(p.0.join("module")).unwrap();
    fs::write(
        p.0.join("module/Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    let source = p.0.join("module/app.rb");
    fs::write(&source, "def f(\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "ruby"])
        .arg(&source)
        .arg("--ruby-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["native"]["reason"], "ruby_project_version_mismatch");
    assert!(!p.0.join("ruby.called").exists());
}
