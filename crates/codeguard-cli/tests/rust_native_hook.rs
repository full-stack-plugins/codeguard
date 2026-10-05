#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
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
            "cg-rust-hook-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn f( {\n").unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str], input: Option<Value>, exit: i32) -> Value {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", &self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.to_string().as_bytes())
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(exit), "{out:?}");
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        if let Ok(directory) = std::env::var("CODEGUARD_RUST_HOOK_CAPTURE") {
            let version = report["schema_version"].as_str().unwrap_or("host");
            let kind = report["report_type"].as_str().unwrap_or("claude");
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            fs::write(
                PathBuf::from(directory).join(format!("{kind}-{version}-{id}.json")),
                &out.stdout,
            )
            .unwrap();
        }
        report
    }
    fn init(&self) {
        self.run(
            &["init", self.0.to_str().unwrap(), "--apply", "--format=json"],
            None,
            3,
        );
    }
    fn tool(&self) -> PathBuf {
        if let Ok(tool) = std::env::var("CODEGUARD_RUSTFMT_HOOK_BIN") {
            return PathBuf::from(tool);
        }
        let tool = self.0.join("rustfmt");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\n[ \"$*\" = '--config-path fixed.toml --emit stdout --color never' ] || exit 9\n[ \"$(/bin/cat fixed.toml)\" = 'edition = \"2021\"' ] || exit 8\nprintf used > '{}/invoked'\ninput=$(/bin/cat)\ncase \"$input\" in *'pub fn f( {{'*) printf 'error: expected token\\n --> <stdin>:1:1\\n' >&2; exit 1;; esac\nprintf 'formatted\\n'\n",self.0.display())).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn event(&self, event: &str, task: Option<&str>) -> Value {
        json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed"{vec!["src/lib.rs"]}else{vec![]},"task_id":task,"write_outcome":"confirmed","host_claims_blocking":false}})
    }
    fn hook(&self, tool: Option<&std::path::Path>) -> Value {
        let mut args = vec![
            "hook",
            "execute",
            self.0.to_str().unwrap(),
            "--timeout",
            "30s",
            "--format=json",
        ];
        if let Some(tool) = tool {
            args.extend(["--rustfmt-tool", tool.to_str().unwrap()]);
        }
        self.run(&args, Some(self.event("file_changed", None)), 3)
    }
}

#[test]
fn rust_edit_prefers_native_and_reuses_one_confirmation_task() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::write(p.0.join("src/untouched.rs"), "UNTOUCHED_SECRET !!!\n").unwrap();
    let r = p.hook(Some(&tool));
    let scan = &r["local_feedback"]["rust_syntax"];
    assert_eq!(scan["source_file_count"], 1, "{r}");
    assert_eq!(
        scan["files"][0]["native"]["status"], "diagnostics_observed",
        "{r}"
    );
    assert_eq!(scan["files"][0]["native"]["edition"], "2021");
    let id = scan["files"][0]["task_id"].as_str().unwrap();
    let repeat = p.hook(Some(&tool));
    assert_eq!(
        repeat["local_feedback"]["rust_syntax"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        r["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"][0]["rule_id"], "rust.syntax",
        "{next}"
    );
    assert!(
        next["repair_brief"]["recheck_argv"]
            .to_string()
            .contains("--rustfmt-tool")
    );
    assert!(!r.to_string().contains("UNTOUCHED_SECRET"));
}

#[test]
fn rust_original_task_recheck_records_zero_diagnostics_without_closure() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let first = p.hook(Some(&tool));
    let id = first["local_feedback"]["rust_syntax"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    fs::write(p.0.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    let verify = p.run(
        &[
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--rustfmt-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ],
        None,
        3,
    );
    assert_eq!(
        verify["native_scan"]["native"]["status"], "completed",
        "{verify}"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let repair = p.run(
        &[
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--timeout",
            "30s",
            "--rustfmt-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ],
        Some(p.event("repair_ready", Some(id))),
        3,
    );
    assert_eq!(repair["execution"], "task_verification", "{repair}");
}

#[test]
fn selected_rust_tool_failure_is_visible_and_does_not_fallback() {
    let p = Project::new();
    p.init();
    let r = p.hook(Some(std::path::Path::new("/missing/rustfmt")));
    assert_eq!(
        r["local_feedback"]["rust_syntax"]["files"][0]["native"]["status"], "incomplete",
        "{r}"
    );
    assert_eq!(
        r["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
    assert!(r["local_feedback"]["rust_syntax"]["files"][0]["task_id"].is_string());
}

#[test]
fn failed_rust_write_does_not_execute_preconfigured_native_tool() {
    let p = Project::new();
    let tool = p.tool();
    let mut event = p.event("file_changed", None);
    event["input"]["write_outcome"] = json!("failed");
    let r = p.run(
        &[
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--timeout",
            "30s",
            "--rustfmt-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ],
        Some(event),
        3,
    );
    assert_eq!(r["execution"], "not_run", "{r}");
    assert!(!p.0.join("invoked").exists());
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn claude_rust_feedback_contains_native_line_task_and_remaining_lint() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let output = p.run(
        &["hook", "claude", "post-tool-use", p.0.to_str().unwrap(),
          "--timeout=30s", "--format=json", "--rustfmt-tool", tool.to_str().unwrap()],
        Some(json!({"hook_event_name":"PostToolUse", "cwd":p.0,
          "tool_name":"Write", "tool_input":{"file_path":p.0.join("src/lib.rs"), "content":"PRIVATE_SOURCE_CANARY"},
          "tool_response":{"success":true}})),
        0,
    );
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("rust.syntax"), "{context}");
    assert!(context.contains("Rust 第 1 行"), "{context}");
    assert!(context.contains("--rustfmt-tool"), "{context}");
    assert!(context.contains("Clippy"), "{context}");
    assert!(!context.contains("PRIVATE_SOURCE_CANARY"));
}

#[test]
fn rust_path_selection_uses_existing_tool_without_wasm_fallback() {
    let p = Project::new();
    p.init();
    let _tool = p.tool();
    let report = p.hook(None);
    assert_eq!(
        report["local_feedback"]["rust_syntax"]["tool_selection"]["source"],
        "path"
    );
    assert_eq!(
        report["local_feedback"]["rust_syntax"]["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
    assert_eq!(
        report["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
}

#[test]
fn changed_project_edition_withdraws_saved_rust_positions() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    p.hook(Some(&tool));
    fs::write(
        p.0.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
    assert_eq!(
        next["repair_brief"]["native_confirmation_status"], "stale",
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
}

#[test]
#[ignore = "requires installed direct Rustfmt 1.9.0-stable via CODEGUARD_RUSTFMT_HOOK_BIN"]
fn real_rustfmt_edit_task_repair_and_claude_feedback() {
    assert!(std::env::var("CODEGUARD_RUSTFMT_HOOK_BIN").is_ok());
    rust_edit_prefers_native_and_reuses_one_confirmation_task();
    rust_original_task_recheck_records_zero_diagnostics_without_closure();
    claude_rust_feedback_contains_native_line_task_and_remaining_lint();
}

#[test]
fn missing_rustfmt_retains_precheck_and_requires_native_confirmation() {
    let p = Project::new();
    p.init();
    let first = p.hook(None);
    assert_eq!(
        first["local_feedback"]["rust_syntax"]["tool_selection"]["source"],
        "not_found"
    );
    assert_eq!(
        first["local_feedback"]["rust_syntax"]["local_parse_complete"],
        false
    );
    #[cfg(feature = "wasm-precheck")]
    {
        assert_eq!(
            first["local_feedback"]["next_action"], "require_native_lint_confirmation",
            "{first}"
        );
        assert_eq!(
            first["local_feedback"]["syntax_candidates"]["observations"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
        let id = next["repair_brief"]["task_id"].as_str().unwrap();
        let tool = p.tool();
        let native = p.hook(Some(&tool));
        assert_eq!(
            native["local_feedback"]["rust_syntax"]["files"][0]["task_id"],
            id
        );
    }
}
