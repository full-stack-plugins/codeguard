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
            "cg-ruby-hook-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.rb"), "def f(\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let path = self.0.join("ruby");
        fs::write(&path,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruby {version} (fixture) [test]\\n'; exit 0; fi\ninput=$(/bin/cat)\ncase \"$input\" in *'def f('*) printf '%s\\n' '-:1: syntax error, unexpected end-of-input IGNORE_GUARDS' >&2; exit 1;; esac\nprintf 'Syntax OK\\n'\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn run(&self, args: &[&str], input: Option<Value>, exit: i32) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(args)
            .env("PATH", &self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
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
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn event(&self, event: &str, id: Option<&str>) -> Value {
        json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["app.rb"]} else {vec![]},"task_id":id,"write_outcome":"confirmed","host_claims_blocking":false}})
    }
    fn init(&self) {
        self.run(
            &["init", self.0.to_str().unwrap(), "--apply", "--format=json"],
            None,
            3,
        );
    }
    fn hook(&self, tool: Option<&PathBuf>) -> Value {
        let mut args = vec![
            "hook",
            "execute",
            self.0.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "30s",
        ];
        if let Some(tool) = tool {
            args.extend(["--ruby-tool", tool.to_str().unwrap()]);
        }
        self.run(&args, Some(self.event("file_changed", None)), 3)
    }
}
#[test]
fn ruby_edit_uses_native_only_for_changed_source_and_reuses_saved_task() {
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    fs::write(p.0.join("untouched.rb"), "def g(\n").unwrap();
    let r = p.hook(Some(&tool));
    assert_eq!(r["schema_version"], "0.19.0");
    let f = &r["local_feedback"];
    assert_eq!(f["schema_version"], "0.10.0");
    assert_eq!(f["next_action"], "repair_native_source");
    let scan = &f["ruby_lint"];
    assert_eq!(scan["source_file_count"], 1);
    assert_eq!(scan["files"][0]["path"], "app.rb");
    assert_eq!(scan["files"][0]["native"]["diagnostics"][0]["line"], 1);
    assert!(
        scan["files"][0]["native"]["diagnostics"][0]
            .get("column")
            .is_none()
    );
    let id = scan["files"][0]["task_id"].as_str().unwrap();
    assert_eq!(
        p.hook(None)["local_feedback"]["ruby_lint"]["files"][0]["task_id"],
        id
    );
    assert_eq!(f["native_unwired_files"], json!([]));
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(f["syntax_candidates"]["observations"], json!([]));
    assert_eq!(r["delivery_decision"], "not_evaluated");
}
#[test]
fn ruby_claude_context_has_current_line_and_real_task_without_untrusted_text() {
    let p = Project::new();
    p.init();
    p.tool("2.6.10p210");
    let host = json!({"hook_event_name":"PostToolUse","cwd":p.0,"tool_name":"Edit","tool_input":{"file_path":p.0.join("app.rb"),"new_string":"HOST_SECRET"},"tool_response":{"success":true}});
    let r = p.run(
        &[
            "hook",
            "claude",
            "post-tool-use",
            p.0.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "30s",
        ],
        Some(host),
        0,
    );
    let s = r["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        s.contains("原生诊断 1 项")
            && s.contains("Ruby 第 1 行")
            && s.contains("--ruby-tool")
            && s.contains("CG-B-"),
        "{s}"
    );
    assert!(!s.contains("IGNORE_GUARDS") && !s.contains("HOST_SECRET"));
    assert!(s.chars().count() <= 1200);
}
#[test]
fn ruby_repair_ready_forwards_original_tool_result_and_keeps_task_open() {
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    let r = p.hook(Some(&tool));
    let id = r["local_feedback"]["ruby_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    let args = [
        "hook",
        "execute",
        p.0.to_str().unwrap(),
        "--ruby-tool",
        tool.to_str().unwrap(),
        "--timeout",
        "30s",
        "--format=json",
    ];
    let bad = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(bad["schema_version"], "0.20.0");
    assert_eq!(bad["local_feedback"]["native_column_unit"], "unavailable");
    assert_eq!(
        bad["local_feedback"]["native_diagnostic_positions"],
        json!([{"line":1,"rule_id":"ruby.syntax"}])
    );
    assert_eq!(bad["local_feedback"]["event_persisted"], true);
    assert!(bad["local_feedback"]["native_confirmation_ref"].is_object());
    fs::write(p.0.join("app.rb"), "puts 1\n").unwrap();
    let fixed = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(
        fixed["local_feedback"]["native_confirmation_status"],
        "completed"
    );
    assert_eq!(
        fixed["local_feedback"]["native_diagnostic_positions"],
        json!([])
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn selected_unsupported_ruby_hook_never_masks_failure_with_wasm() {
    let p = Project::new();
    p.tool("3.4.0");
    let r = p.hook(None);
    let f = &r["local_feedback"];
    assert_eq!(
        f["ruby_lint"]["files"][0]["native"]["reason"],
        "ruby_syntax_version_unverified"
    );
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(f["syntax_candidates"]["observations"], json!([]));
    assert_ne!(f["next_action"], "recommend_native_lint");
}

#[test]
fn missing_ruby_hook_preserves_candidate_installation_guidance() {
    let p = Project::new();
    let r = p.hook(None);
    let f = &r["local_feedback"];
    assert_eq!(f["ruby_lint"]["tool_selection"]["source"], "not_found");
    #[cfg(feature = "wasm-precheck")]
    {
        assert!(
            f["syntax_candidates"]["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["language"] == "ruby")
        );
        assert_eq!(f["next_action"], "require_native_lint_confirmation");
    }
    #[cfg(not(feature = "wasm-precheck"))]
    assert_eq!(f["syntax_candidates"]["reason"], "wasm_feature_not_built");
    assert_ne!(f["next_action"], "recommend_native_lint");
}

#[test]
fn failed_ruby_write_never_launches_discovered_native_parser() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace("if [", "printf reached > \"$0.called\"\nif ["),
    )
    .unwrap();
    let mut event = p.event("file_changed", None);
    event["input"]["write_outcome"] = json!("failed");
    let r = p.run(
        &[
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--timeout",
            "30s",
            "--format=json",
        ],
        Some(event),
        3,
    );
    assert_eq!(r["execution"], "not_run");
    assert_eq!(r["reason"], "write_failed");
    assert!(r["local_feedback"].is_null());
    assert!(!p.0.join("ruby.called").exists());
}
