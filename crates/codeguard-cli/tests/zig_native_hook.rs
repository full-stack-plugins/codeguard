#![cfg(unix)]
use serde_json::Value;
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
            "cg-zig-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), "pub fn main( void {\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("zig");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = version ]; then printf '{version}\\n'; exit 0; fi\nwhile IFS= read -r line; do :; done\nprintf '<stdin>:1:13: error: failure\\n' >&2\nexit 1\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn hook(&self, explicit: bool, claude: bool) -> Value {
        self.hook_paths(explicit, claude, &["app.zig"])
    }
    fn hook_paths(&self, explicit: bool, claude: bool, paths: &[&str]) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env("PATH", &self.0);
        if claude {
            c.args(["hook", "claude", "post-tool-use"]);
        } else {
            c.args(["hook", "execute"]);
        }
        c.arg(&self.0).args(["--format=json", "--timeout", "30s"]);
        if explicit {
            c.arg("--zig-tool").arg(self.0.join("zig"));
        }
        c.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let input = if claude {
            serde_json::json!({"hook_event_name":"PostToolUse","cwd":self.0,"tool_name":"Edit","tool_input":{"file_path":self.0.join("app.zig"),"old_string":"old","new_string":"new"},"tool_response":{"success":true}})
        } else {
            serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":paths,"write_outcome":"confirmed","host_claims_blocking":false}})
        };
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if claude { 0 } else { 3 }),
            "{out:?}"
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn confirmed_zig_edit_checks_only_the_requested_file_with_native_tool() {
    let p = Project::new();
    p.tool("0.16.0");
    fs::write(p.0.join("untouched.zig"), "pub fn broken( void {\n").unwrap();
    let r = p.hook(true, false);
    let f = &r["local_feedback"];
    assert_eq!(f["zig_lint"]["files"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(f["zig_lint"]["files"][0]["path"], "app.zig");
    assert_eq!(
        f["zig_lint"]["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
    assert_eq!(f["next_action"], "repair_native_source");
    assert!(f["zig_lint"]["files"][0]["task_id"].is_null());
    #[cfg(feature = "wasm-precheck")]
    assert!(
        f["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["language"] != "zig")
    );
}
#[test]
fn claude_feedback_contains_native_rule_position_and_original_recheck() {
    let p = Project::new();
    p.tool("0.16.0");
    let r = p.hook(false, true);
    let context = r["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("Zig 当前原生位置 1:13"), "{r}");
    assert!(context.contains("codeguard lint zig"));
    assert!(context.contains("zig.ast_check.error"), "{context}");
}
