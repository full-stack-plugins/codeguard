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
            "cg-shell-hook-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.sh"), "#!/bin/bash\necho $1\n").unwrap();
        Self(root)
    }
    fn tool(&self) -> PathBuf {
        let path = self.0.join("shellcheck");
        fs::write(&path, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: 0.11.0\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi\ninput=$(/bin/cat)\ncase \"$input\" in *'echo $1'*) printf '%s\\n' '{\"comments\":[{\"file\":\"-\",\"line\":2,\"endLine\":2,\"column\":6,\"endColumn\":8,\"level\":\"info\",\"code\":2086,\"message\":\"IGNORE_GUARDS\",\"fix\":null}]}'; exit 1;; esac\nprintf '%s\\n' '{\"comments\":[]}'\n").unwrap();
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
        json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["app.sh"]} else {vec![]},"task_id":id,"write_outcome":"confirmed","host_claims_blocking":false}})
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
            args.extend(["--shellcheck-tool", tool.to_str().unwrap()]);
        }
        self.run(&args, Some(self.event("file_changed", None)), 3)
    }
}
#[test]
fn shell_edit_checks_only_changed_source_and_updates_one_saved_task() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::write(p.0.join("untouched.sh"), "#!/bin/bash\necho $1\n").unwrap();
    let report = p.hook(Some(&tool));
    assert_eq!(report["schema_version"], "0.21.0");
    let f = &report["local_feedback"];
    assert_eq!(f["schema_version"], "0.11.0");
    assert_eq!(f["next_action"], "repair_native_source");
    let scan = &f["shell_lint"];
    assert_eq!(scan["source_file_count"], 1);
    assert_eq!(scan["files"][0]["path"], "app.sh");
    assert_eq!(
        scan["files"][0]["native"]["diagnostics"][0]["rule_id"],
        "SC2086"
    );
    let ids = &scan["files"][0]["workbench"]["task_ids"];
    assert_eq!(ids.as_array().unwrap().len(), 1);
    assert_eq!(
        p.hook(None)["local_feedback"]["shell_lint"]["files"][0]["workbench"]["task_ids"],
        *ids
    );
    assert_eq!(f["native_unwired_files"], json!([]));
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
#[test]
fn shell_repair_ready_uses_original_rule_and_never_closes_on_zero_diagnostics() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let report = p.hook(Some(&tool));
    let id = report["local_feedback"]["shell_lint"]["files"][0]["workbench"]["task_ids"][0]
        .as_str()
        .unwrap();
    let args = [
        "hook",
        "execute",
        p.0.to_str().unwrap(),
        "--shellcheck-tool",
        tool.to_str().unwrap(),
        "--timeout",
        "30s",
        "--format=json",
    ];
    let bad = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(bad["local_feedback"]["observation"], "still_present");
    assert_eq!(bad["local_feedback"]["event_persisted"], true);
    fs::write(p.0.join("app.sh"), "#!/bin/bash\necho \"$1\"\n").unwrap();
    let fixed = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(
        fixed["local_feedback"]["observation"],
        "candidate_absent_unverified_policy"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn shell_claude_context_has_current_character_positions_and_safe_task_id() {
    let p = Project::new();
    p.init();
    p.tool();
    let host = json!({"hook_event_name":"PostToolUse","cwd":p.0,"tool_name":"Edit","tool_input":{"file_path":p.0.join("app.sh"),"new_string":"HOST_SECRET"},"tool_response":{"success":true}});
    let report = p.run(
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
    let text = report["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("原生诊断 1 项")
            && text.contains("SC2086")
            && text.contains("Shell 第 2 行")
            && text.contains("--shellcheck-tool")
            && text.contains("CG-"),
        "{text}"
    );
    assert!(!text.contains("IGNORE_GUARDS") && !text.contains("HOST_SECRET"));
    assert!(text.chars().count() <= 1200);
}
#[test]
fn shell_missing_tool_retains_environment_feedback_without_inventing_wasm() {
    let p = Project::new();
    let report = p.hook(None);
    let f = &report["local_feedback"];
    assert_eq!(
        f["shell_lint"]["files"][0]["native"]["reason"],
        "shellcheck_tool_not_found"
    );
    assert_ne!(f["next_action"], "recommend_native_lint");
    assert_eq!(f["native_unwired_files"], json!([]));
    assert!(!p.0.join(".codeguard").exists());
}
#[test]
fn shell_unsupported_dialect_and_selected_tool_failure_remain_incomplete() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::write(p.0.join("app.sh"), "#!/bin/zsh\necho $1\n").unwrap();
    let unsupported = p.hook(Some(&tool));
    assert_eq!(
        unsupported["local_feedback"]["shell_lint"]["files"][0]["native"]["reason"],
        "shell_dialect_unsupported"
    );
    assert_ne!(
        unsupported["local_feedback"]["next_action"],
        "repair_native_source"
    );
    fs::write(p.0.join("app.sh"), "#!/bin/bash\necho $1\n").unwrap();
    fs::write(&tool, "#!/bin/sh\nexit 7\n").unwrap();
    let failed = p.hook(Some(&tool));
    assert_eq!(
        failed["local_feedback"]["shell_lint"]["files"][0]["native"]["status"],
        "incomplete"
    );
    assert_ne!(
        failed["local_feedback"]["next_action"],
        "recommend_native_lint"
    );
    assert_eq!(
        failed["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
}
#[test]
fn shell_failed_write_runs_no_check_and_creates_no_tasks() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let mut event = p.event("file_changed", None);
    event["input"]["write_outcome"] = json!("failed");
    let report = p.run(
        &[
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--timeout",
            "30s",
            "--shellcheck-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ],
        Some(event),
        3,
    );
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["reason"], "write_failed");
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        0
    );
}
