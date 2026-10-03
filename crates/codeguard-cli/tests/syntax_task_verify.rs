#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new() -> (Self, String) {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-native-syntax-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), "pub fn main( void {\n").unwrap();
        let p = Self(root);
        let o = p
            .command()
            .arg("init")
            .arg(&p.0)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let mut c = p.command();
        c.args(["hook", "execute"])
            .arg(&p.0)
            .args(["--timeout", "30s", "--format=json"])
            .env("PATH", &p.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let request = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.zig"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(request.to_string().as_bytes())
            .unwrap();
        let o = child.wait_with_output().unwrap();
        let r: Value = serde_json::from_slice(&o.stdout).unwrap();
        let id = r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        (p, id)
    }
    fn command(&self) -> Command {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
    }
    fn tool(&self, body: &str) -> PathBuf {
        let p = self.0.join("zig-tool");
        fs::write(
            &p,
            format!(
                "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\n{body}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn verify(&self, id: &str, tool: Option<&std::path::Path>) -> (i32, Value) {
        let mut c = self.command();
        c.args(["task", "verify", id])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(tool) = tool {
            c.arg("--zig-tool").arg(tool);
        }
        let o = c.output().unwrap();
        (
            o.status.code().unwrap(),
            serde_json::from_slice(&o.stdout)
                .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stderr))),
        )
    }
    fn next(&self) -> Value {
        let o = self
            .command()
            .arg("next")
            .arg(&self.0)
            .arg("--format=json")
            .output()
            .unwrap();
        serde_json::from_slice(&o.stdout).unwrap()
    }
    fn task_operation(&self, args: &[&str]) -> (i32, Value) {
        let split = if args[1] == "attempt" { 4 } else { 3 };
        let o = self
            .command()
            .args(&args[..split])
            .arg(&self.0)
            .args(&args[split..])
            .arg("--format=json")
            .output()
            .unwrap();
        (
            o.status.code().unwrap(),
            serde_json::from_slice(&o.stdout)
                .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stderr))),
        )
    }
}

#[test]
fn native_diagnostics_enter_existing_verification_events_and_guide_source_repair() {
    let (p, id) = Project::new();
    let tool=p.tool("while IFS= read -r line; do :; done\nprintf '<stdin>:1:13: error: expected token\\n' >&2\nexit 1");
    let (exit, r) = p.verify(&id, Some(&tool));
    assert_eq!(exit, 3);
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "still_blocked");
    assert_eq!(r["native_scan"]["native"]["status"], "diagnostics_observed");
    let next = p.next();
    assert_eq!(next["repair_brief"]["disposition"], "actionable", "{next}");
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("原生")
    );
    let argv = next["repair_brief"]["recheck_argv"].as_array().unwrap();
    assert_eq!(argv[argv.len() - 2], "--zig-tool");
    assert_eq!(argv[argv.len() - 1], tool.to_str().unwrap());
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"][0]["column"],
        13
    );
    assert_eq!(
        next["repair_brief"]["native_confirmation_ref"]["run_id"],
        r["native_scan"]["run_id"]
    );
    fs::write(p.0.join("app.zig"), "pub fn main() void {}\n").unwrap();
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["disposition"], "verification_required",
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
}

#[test]
fn repair_ready_hook_accepts_the_native_confirmation_tool() {
    let (p, id) = Project::new();
    let tool = p.tool("while IFS= read -r line; do :; done\nexit 0");
    let mut c = p.command();
    c.args(["hook", "execute"])
        .arg(&p.0)
        .arg("--zig-tool")
        .arg(&tool)
        .args(["--timeout", "30s", "--format=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let request = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"repair_ready","changed_paths":[],"task_id":id,"write_outcome":"unknown","host_claims_blocking":false}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(
        o.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r["local_feedback"]["event_persisted"], true, "{r}");
    assert_eq!(
        r["local_feedback"]["observation"], "candidate_absent_unverified_policy",
        "{r}"
    );
    assert_eq!(r["delivery_decision"], "not_evaluated");
}

#[test]
fn native_zero_diagnostics_are_recorded_without_closing_the_task() {
    let (p, id) = Project::new();
    let tool = p.tool("while IFS= read -r line; do :; done\nexit 0");
    fs::write(p.0.join("app.zig"), "pub fn main() void {}\n").unwrap();
    let (_, r) = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "candidate_absent_unverified_policy");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(&tool, "changed").unwrap();
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["disposition"], "verification_required",
        "{next}"
    );
}

#[test]
fn missing_tool_still_records_a_failed_native_confirmation_observation() {
    let (p, id) = Project::new();
    let (_, r) = p.verify(&id, None);
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "incomplete");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "explicit_zig_tool_not_provided"
    );
}

#[test]
fn native_recheck_binds_the_finished_attempt_and_preserves_the_owned_lease() {
    let (p, id) = Project::new();
    let tool = p.tool("bad=0\nwhile IFS= read -r line; do case \"$line\" in *'main( void'*) bad=1;; esac; done\nif [ \"$bad\" = 1 ]; then printf '<stdin>:1:13: error: expected token\\n' >&2; exit 1; fi\nexit 0");
    let (_, initial) = p.verify(&id, Some(&tool));
    assert_eq!(initial["event_persisted"], true, "{initial}");
    assert_eq!(initial["observation"], "still_blocked");
    let (_, lease) = p.task_operation(&["task", "claim", &id, "--owner", "agent-a"]);
    let token = lease["lease_token"].as_str().unwrap();
    let action = p.next()["repair_brief"]["action_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (exit, start) = p.task_operation(&[
        "task",
        "attempt",
        "start",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        &action,
    ]);
    assert_eq!(exit, 0, "{start}");
    let attempt = start["attempt_id"].as_str().unwrap();
    fs::write(p.0.join("app.zig"), "pub fn main() void {}\n").unwrap();
    let (exit, finish) = p.task_operation(&[
        "task",
        "attempt",
        "finish",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--attempt-id",
        attempt,
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "source_edit",
    ]);
    assert_eq!(exit, 0, "{finish}");
    assert_eq!(
        p.next()["repair_brief"]["history"]["awaiting_verification"],
        true
    );
    let (exit, verify) = p.task_operation(&[
        "task",
        "verify",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--zig-tool",
        tool.to_str().unwrap(),
    ]);
    assert_eq!(exit, 3);
    assert_eq!(verify["event_persisted"], true, "{verify}");
    let run = verify["native_scan"]["run_id"].as_str().unwrap();
    let event: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/events/verify-{run}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(event["attempt_id"], attempt);
    assert_eq!(
        p.next()["repair_brief"]["history"]["awaiting_verification"],
        false
    );
    let (exit, _) = p.task_operation(&[
        "task",
        "heartbeat",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
    ]);
    assert_eq!(exit, 0, "verification must retain the caller's lease");
}

#[test]
fn native_environment_failures_do_not_recommend_source_repair() {
    let (p, id) = Project::new();
    let tool = p.tool(
        "while IFS= read -r line; do :; done\nprintf 'tool environment failed\\n' >&2\nexit 1",
    );
    let (_, r) = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "incomplete");
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["action_id"], "restore-checker-environment",
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["disposition"], "needs_decision",
        "{next}"
    );
    fs::write(&tool, "#!/bin/sh\nprintf '0.15.0\\n'\n").unwrap();
    let (_, r) = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "zig_version_unverified_or_unsupported"
    );
}

#[test]
fn repeated_no_change_native_repairs_stop_with_a_specific_decision() {
    let (p, id) = Project::new();
    let tool=p.tool("while IFS= read -r line; do :; done\nprintf '<stdin>:1:13: error: expected token\\n' >&2\nexit 1");
    assert_eq!(p.verify(&id, Some(&tool)).1["event_persisted"], true);
    let (_, lease) = p.task_operation(&["task", "claim", &id, "--owner", "agent-a"]);
    let token = lease["lease_token"].as_str().unwrap();
    for _ in 0..2 {
        let (exit, start) = p.task_operation(&[
            "task",
            "attempt",
            "start",
            &id,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--action-id",
            "repair-source",
        ]);
        assert_eq!(exit, 0, "{start}");
        let attempt = start["attempt_id"].as_str().unwrap();
        let (exit, finish) = p.task_operation(&[
            "task",
            "attempt",
            "finish",
            &id,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--attempt-id",
            attempt,
            "--outcome",
            "no-change",
            "--note-code",
            "no_change",
        ]);
        assert_eq!(exit, 0, "{finish}");
    }
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["disposition"], "needs_decision",
        "{next}"
    );
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 2);
    let (exit, start) = p.task_operation(&[
        "task",
        "attempt",
        "start",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        "repair-source",
    ]);
    assert_eq!(exit, 3);
    assert_eq!(start["reason"], "no_progress_budget_exhausted");
}

#[test]
fn source_changes_during_native_confirmation_cannot_authorize_repair_from_old_positions() {
    let (p, id) = Project::new();
    let tool=p.tool("while IFS= read -r line; do :; done\nprintf 'pub fn main() void {}\\n' > app.zig\nprintf '<stdin>:1:13: error: expected token\\n' >&2\nexit 1");
    let (_, r) = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "incomplete");
    assert_eq!(r["native_scan"]["input_stable"], false);
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["disposition"], "verification_required",
        "{next}"
    );
    assert_eq!(next["repair_brief"]["native_confirmation_status"], "stale");
}

#[test]
#[ignore = "requires existing native Zig 0.16.0 via CODEGUARD_ZIG_BIN"]
fn real_native_zig_rechecks_broken_then_repaired_source() {
    let zig = PathBuf::from(std::env::var("CODEGUARD_ZIG_BIN").unwrap());
    let (p, id) = Project::new();
    let (_, bad) = p.verify(&id, Some(&zig));
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(
        bad["native_scan"]["native"]["status"],
        "diagnostics_observed"
    );
    fs::write(p.0.join("app.zig"), "pub fn main() void {}\n").unwrap();
    let (_, good) = p.verify(&id, Some(&zig));
    assert_eq!(good["event_persisted"], true, "{good}");
    assert_eq!(good["observation"], "candidate_absent_unverified_policy");
}
