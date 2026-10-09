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
            "cg-go-hook-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.go"), "package main\nfunc main( {\n").unwrap();
        Self(root)
    }
    fn tool(&self) -> PathBuf {
        let path = self.0.join("go");
        fs::write(&path, "#!/bin/sh\nif [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi\n").unwrap();
        let fmt = self.0.join("gofmt");
        fs::write(&fmt,"#!/bin/sh\ninput=$(/bin/cat)\ncase \"$input\" in *'func main( {'*) printf '/dev/stdin:2:12: private-message\\n' >&2; exit 2;; esac\nprintf '%s\\n' \"$input\"\n").unwrap();
        fs::set_permissions(&fmt, fs::Permissions::from_mode(0o700)).unwrap();
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
        json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["app.go"]} else {vec![]},"task_id":id,"write_outcome":"confirmed","host_claims_blocking":false}})
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
            args.extend(["--go-tool", tool.to_str().unwrap()]);
        }
        self.run(&args, Some(self.event("file_changed", None)), 3)
    }
}
#[test]
fn go_edit_prefers_native_and_reuses_one_task() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::write(p.0.join("untouched.go"), "package main\nfunc main( {\n").unwrap();
    let report = p.hook(Some(&tool));
    let scan = &report["local_feedback"]["go_syntax"];
    assert_eq!(scan["source_file_count"], 1, "{report}");
    assert_eq!(
        scan["files"][0]["native"]["status"], "diagnostics_observed",
        "{report}"
    );
    let id = scan["files"][0]["task_id"].as_str().unwrap();
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
    assert_eq!(next["schema_version"], "0.18.0", "{next}");
    assert!(
        next["repair_brief"]["native_confirmation_ref"]["run_id"]
            .as_str()
            .unwrap()
            .starts_with("syntax-confirm-")
    );
    assert_eq!(
        p.hook(None)["local_feedback"]["go_syntax"]["files"][0]["task_id"],
        id
    );
    assert_eq!(
        report["local_feedback"]["next_action"],
        "repair_native_source"
    );
    assert_eq!(
        report["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let args = [
        "hook",
        "execute",
        p.0.to_str().unwrap(),
        "--go-tool",
        tool.to_str().unwrap(),
        "--timeout",
        "30s",
        "--format=json",
    ];
    let present = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(
        present["local_feedback"]["observation"], "still_blocked",
        "{present}"
    );
    assert_eq!(
        present["local_feedback"]["event_persisted"], true,
        "{present}"
    );
    fs::write(p.0.join("app.go"), "package main\nfunc main() {}\n").unwrap();
    let fixed = p.run(&args, Some(p.event("repair_ready", Some(id))), 3);
    assert_eq!(
        fixed["local_feedback"]["observation"], "candidate_absent_unverified_policy",
        "{fixed}"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn go_selected_failure_never_switches_to_wasm() {
    let p = Project::new();
    let tool = p.tool();
    fs::write(&tool, "#!/bin/sh\nexit 7\n").unwrap();
    let report = p.hook(Some(&tool));
    assert_eq!(
        report["local_feedback"]["go_syntax"]["files"][0]["native"]["status"], "incomplete",
        "{report}"
    );
    assert_eq!(
        report["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
}
#[test]
fn go_missing_native_retains_fallback_and_explicit_requirement() {
    let p = Project::new();
    let report = p.hook(None);
    let scan = &report["local_feedback"]["go_syntax"];
    assert_eq!(scan["tool_selection"]["source"], "not_found", "{report}");
    assert_eq!(scan["local_parse_complete"], false);
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(
        report["local_feedback"]["next_action"], "require_native_lint_confirmation",
        "{report}"
    );
}

#[test]
fn go_claude_context_has_byte_positions_and_safe_task_reference() {
    let p = Project::new();
    p.init();
    p.tool();
    let host = json!({"hook_event_name":"PostToolUse","cwd":p.0,"tool_name":"Edit","tool_input":{"file_path":p.0.join("app.go"),"new_string":"HOST_SECRET"},"tool_response":{"success":true}});
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
    let context = report["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        context.contains("Go 第 2 行")
            && context.contains("UTF-8字节")
            && context.contains("--go-tool")
            && context.contains("CG-B-"),
        "{context}"
    );
    assert!(!context.contains("private-message") && !context.contains("HOST_SECRET"));
    assert!(context.chars().count() <= 1200);
}
#[test]
fn go_missing_companion_and_logical_positions_are_environment_incomplete() {
    let p = Project::new();
    let tool = p.tool();
    fs::remove_file(p.0.join("gofmt")).unwrap();
    let blocked = p.hook(Some(&tool));
    assert_eq!(
        blocked["local_feedback"]["go_syntax"]["files"][0]["native"]["reason"],
        "go_syntax_tool_unavailable"
    );
    assert_eq!(
        blocked["local_feedback"]["syntax_candidates"]["observations"],
        json!([])
    );
    p.tool();
    fs::write(
        p.0.join("app.go"),
        "package main\n//line other.go:1\nfunc main( {\n",
    )
    .unwrap();
    let logical = p.hook(Some(&tool));
    assert_eq!(
        logical["local_feedback"]["go_syntax"]["files"][0]["native"]["reason"],
        "go_syntax_logical_positions_unresolved"
    );
    assert_eq!(
        logical["local_feedback"]["go_syntax"]["files"][0]["native"]["diagnostics"],
        json!([])
    );
}

#[test]
fn incompatible_go_module_is_environment_blocker_without_invoking_sdk() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let marker = p.0.join("invoked");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf x > '{}'\nprintf 'go version go1.23.4 fixture/fixture\\n'\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::write(
        p.0.join("go.mod"),
        "module example.invalid/sample\ngo 1.24.0\n",
    )
    .unwrap();
    let report = p.hook(Some(&tool));
    let native = &report["local_feedback"]["go_syntax"]["files"][0]["native"];
    assert_eq!(native["status"], "incomplete", "{report}");
    assert_eq!(native["reason"], "go_project_version_mismatch", "{report}");
    assert_eq!(native["diagnostics"], json!([]));
    assert!(!marker.exists(), "incompatible SDK must not be executed");
    assert_eq!(
        report["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn nearest_module_and_workspace_have_separate_version_boundaries() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::create_dir(p.0.join("child")).unwrap();
    fs::write(p.0.join("child/app.go"), "package main\nfunc main( {\n").unwrap();
    fs::write(p.0.join("go.mod"), "module parent\ngo 1.24.0\n").unwrap();
    fs::write(
        p.0.join("child/go.mod"),
        "module child\ngo 1.22\ntoolchain go1.23.4\n",
    )
    .unwrap();
    let observe = || {
        let mut event = p.event("file_changed", None);
        event["input"]["changed_paths"] = json!(["child/app.go"]);
        p.run(
            &[
                "hook",
                "execute",
                p.0.to_str().unwrap(),
                "--go-tool",
                tool.to_str().unwrap(),
                "--format=json",
                "--timeout",
                "30s",
            ],
            Some(event),
            3,
        )
    };
    assert_eq!(
        observe()["local_feedback"]["go_syntax"]["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
    fs::write(p.0.join("go.work"), "go 1.24\nuse ./child\n").unwrap();
    assert_eq!(
        observe()["local_feedback"]["go_syntax"]["files"][0]["native"]["reason"],
        "go_project_version_mismatch"
    );
    fs::write(p.0.join("child/go.work"), "go 1.23\nuse .\n").unwrap();
    assert_eq!(
        observe()["local_feedback"]["go_syntax"]["files"][0]["native"]["status"],
        "diagnostics_observed"
    );
}

#[test]
fn duplicate_unreadable_and_newer_toolchain_declarations_do_not_create_source_diagnostics() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    for (text, reason) in [
        (
            "module sample\ngo 1.23\ngo 1.22\n",
            "go_project_version_unresolved",
        ),
        (
            "module sample\ngo 1.23\ntoolchain go1.24.0\n",
            "go_project_toolchain_mismatch",
        ),
        (
            "module sample\ngo 1.23rc1\n",
            "go_project_version_unresolved",
        ),
    ] {
        fs::write(p.0.join("go.mod"), text).unwrap();
        let report = p.hook(Some(&tool));
        let native = &report["local_feedback"]["go_syntax"]["files"][0]["native"];
        assert_eq!(native["reason"], reason, "{report}");
        assert_eq!(native["diagnostics"], json!([]));
    }
    fs::remove_file(p.0.join("go.mod")).unwrap();
    std::os::unix::fs::symlink(p.0.join("app.go"), p.0.join("go.mod")).unwrap();
    assert_eq!(
        p.hook(Some(&tool))["local_feedback"]["go_syntax"]["files"][0]["native"]["reason"],
        "go_project_version_unreadable"
    );
}

#[test]
fn changed_go_version_withdraws_observation_and_saved_repair_positions() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    fs::write(p.0.join("go.mod"), "module sample\ngo 1.23\n").unwrap();
    let first = p.hook(Some(&tool));
    let id = first["local_feedback"]["go_syntax"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    fs::write(p.0.join("go.mod"), "module sample\ngo 1.24\n").unwrap();
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
    assert_ne!(
        next["repair_brief"]["native_confirmation_status"], "diagnostics_observed",
        "{next}"
    );
    let verify = p.run(
        &[
            "hook",
            "execute",
            p.0.to_str().unwrap(),
            "--go-tool",
            tool.to_str().unwrap(),
            "--format=json",
            "--timeout",
            "30s",
        ],
        Some(p.event("repair_ready", Some(id))),
        3,
    );
    assert_eq!(
        verify["local_feedback"]["observation"], "incomplete",
        "{verify}"
    );
    let brief = p.run(&["next", p.0.to_str().unwrap(), "--format=json"], None, 0);
    assert!(
        brief.to_string().contains("go_project_version_mismatch"),
        "{brief}"
    );
    fs::write(p.0.join("go.mod"), "module sample\ngo 1.23\n").unwrap();
    let script = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        script.replacen(
            "#!/bin/sh\n",
            &format!(
                "#!/bin/sh\nprintf 'module sample\\ngo 1.24\\n' > '{}'\n",
                p.0.join("go.mod").display()
            ),
            1,
        ),
    )
    .unwrap();
    let mutated = p.hook(Some(&tool));
    let native = &mutated["local_feedback"]["go_syntax"]["files"][0]["native"];
    assert_eq!(
        native["reason"], "go_project_version_changed_during_check",
        "{mutated}"
    );
    assert_eq!(native["diagnostics"], json!([]));
}
