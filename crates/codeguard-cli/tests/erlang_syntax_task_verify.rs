#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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
    fn new() -> (Self, String) {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-erl-task-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.erl"), "-module(app).\nf( -> ok.\n").unwrap();
        let p = Self(root);
        let out = p
            .command()
            .arg("init")
            .arg(&p.0)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let report = p.hook("file_changed", None, None);
        let id = report["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{report}"))
            .to_owned();
        (p, id)
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .env("PATH", self.0.join("empty-path"));
        c
    }
    fn hook(&self, event: &str, task: Option<&str>, tool: Option<&Path>) -> Value {
        self.hook_with_path(event, task, tool, None)
    }
    fn hook_with_path(
        &self,
        event: &str,
        task: Option<&str>,
        tool: Option<&Path>,
        path: Option<&std::ffi::OsStr>,
    ) -> Value {
        let mut c = self.command();
        c.args(["hook", "execute"])
            .arg(&self.0)
            .args(["--timeout", "30s", "--format=json"]);
        if let Some(t) = tool {
            c.arg("--erl-tool").arg(t);
        }
        if let Some(path) = path {
            c.env("PATH", path);
        }
        c.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let req = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["app.erl"]} else {vec![]},"task_id":task,"write_outcome":"confirmed","host_claims_blocking":false}});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(req.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        decode(&out)
    }
    fn tool(&self, body: &str) -> PathBuf {
        let tool = self.0.join("erl-tool");
        fs::write(&tool, format!("#!/bin/sh\ncase \"$*\" in *system_info*) printf 'OTP 28\\n'; exit 0;; esac\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn verify(&self, id: &str, tool: Option<&Path>) -> Value {
        let mut c = self.command();
        c.args(["task", "verify", id])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(t) = tool {
            c.arg("--erl-tool").arg(t);
        }
        let out = c.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        decode(&out)
    }
    fn next(&self) -> Value {
        decode(
            &self
                .command()
                .arg("next")
                .arg(&self.0)
                .arg("--format=json")
                .output()
                .unwrap(),
        )
    }
    fn verify_with_path(&self, id: &str, path: &std::ffi::OsStr, tool: Option<&Path>) -> Value {
        let mut c = self.command();
        c.args(["task", "verify", id])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", path)
            .current_dir(&self.0);
        if let Some(tool) = tool {
            c.arg("--erl-tool").arg(tool);
        }
        let output = c.output().unwrap();
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        decode(&output)
    }
    fn path_tool(&self, directory: &str, version: &str, body: &str) -> PathBuf {
        let directory = self.0.join(directory);
        fs::create_dir_all(&directory).unwrap();
        let tool = directory.join("erl");
        fs::write(&tool, format!("#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' '{version}'; exit 0;; esac\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn operation(&self, args: &[&str]) -> Value {
        let split = if args[1] == "attempt" { 4 } else { 3 };
        let out = self
            .command()
            .args(&args[..split])
            .arg(&self.0)
            .args(&args[split..])
            .arg("--format=json")
            .output()
            .unwrap();
        assert!(
            matches!(out.status.code(), Some(0 | 3)),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        decode(&out)
    }
}
fn decode(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}
const PARSE_BY_SOURCE: &str = "input=$(/bin/cat)\ncase \"$input\" in *'f( ->'*) printf '%s' '{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":[{\"line\":2,\"column\":4,\"rule_id\":\"erlang.syntax.error\"}]}';; *) printf '%s' '{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":[]}';; esac";

#[test]
fn path_recheck_finds_native_diagnostics_and_retains_original_tool_in_next() {
    let (p, id) = Project::new();
    let tool = p.path_tool("first", "OTP 28", PARSE_BY_SOURCE);
    let report = p.verify_with_path(&id, tool.parent().unwrap().as_os_str(), None);
    assert_eq!(report["event_persisted"], true, "{report}");
    assert_eq!(report["observation"], "still_blocked");
    assert_eq!(report["native_scan"]["tool_path"], tool.to_str().unwrap());
    let next = p.next();
    assert_eq!(next["repair_brief"]["disposition"], "actionable");
    let argv = next["repair_brief"]["recheck_argv"].as_array().unwrap();
    assert_eq!(argv[argv.len() - 2], "--erl-tool");
    assert_eq!(argv[argv.len() - 1], tool.to_str().unwrap());
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let other = p.path_tool("other", "OTP 29", "exit 9");
    let fixed = p.verify_with_path(&id, other.parent().unwrap().as_os_str(), Some(&tool));
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    assert_eq!(fixed["native_scan"]["tool_path"], tool.to_str().unwrap());
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn path_recheck_does_not_replace_explicit_bad_or_unsupported_first_tool() {
    let (p, id) = Project::new();
    let marker = p.0.join("second-started");
    let second = p.path_tool(
        "second",
        "OTP 28",
        &format!("/usr/bin/touch '{}'\n{PARSE_BY_SOURCE}", marker.display()),
    );
    let absent = p.0.join("absent");
    let explicit = p.verify_with_path(&id, second.parent().unwrap().as_os_str(), Some(&absent));
    assert_eq!(explicit["observation"], "incomplete");
    assert_eq!(
        explicit["native_scan"]["native"]["reason"],
        "erlang_tool_unavailable_or_untrusted"
    );
    assert!(!marker.exists());
    let first = p.path_tool("first", "OTP 29", "exit 9");
    let path = std::env::join_paths([first.parent().unwrap(), second.parent().unwrap()]).unwrap();
    let unsupported = p.verify_with_path(&id, &path, None);
    assert_eq!(unsupported["event_persisted"], true);
    assert_eq!(
        unsupported["native_scan"]["tool_path"],
        first.to_str().unwrap()
    );
    assert_eq!(
        unsupported["native_scan"]["native"]["reason"],
        "erlang_version_unverified_or_unsupported"
    );
    assert!(!marker.exists());
}

#[test]
fn path_recheck_keeps_selected_execution_failure_and_ignores_unsafe_entries() {
    let (p, id) = Project::new();
    let first = p.path_tool("first", "OTP 28", "exit 9");
    let second = p.path_tool("second", "OTP 28", PARSE_BY_SOURCE);
    let path = std::env::join_paths([first.parent().unwrap(), second.parent().unwrap()]).unwrap();
    let failed = p.verify_with_path(&id, &path, None);
    assert_eq!(failed["event_persisted"], true);
    assert_eq!(failed["observation"], "incomplete");
    assert_eq!(failed["native_scan"]["tool_path"], first.to_str().unwrap());
    fs::set_permissions(&second, fs::Permissions::from_mode(0o600)).unwrap();
    let unsafe_path =
        std::env::join_paths([Path::new(""), Path::new("first"), second.parent().unwrap()])
            .unwrap();
    let absent = p.verify_with_path(&id, &unsafe_path, None);
    assert_eq!(absent["event_persisted"], true);
    assert_eq!(absent["native_scan"]["tool_path"], Value::Null);
    assert_eq!(
        absent["native_scan"]["native"]["reason"],
        "erlang_tool_not_found_on_path"
    );
}

#[test]
fn repair_ready_discovers_path_erlang_and_records_native_verification() {
    let (p, id) = Project::new();
    let tool = p.path_tool("hook-bin", "OTP 28", PARSE_BY_SOURCE);
    let hook = p.hook_with_path(
        "repair_ready",
        Some(&id),
        None,
        Some(tool.parent().unwrap().as_os_str()),
    );
    let report = &hook["local_feedback"];
    assert_eq!(report["event_persisted"], true, "{hook}");
    assert_eq!(report["observation"], "still_blocked");
    assert_eq!(report["native_confirmation_status"], "diagnostics_observed");
    let reference = report["native_confirmation_ref"]["report_ref"]
        .as_str()
        .unwrap();
    let saved: Value = serde_json::from_slice(&fs::read(p.0.join(reference)).unwrap()).unwrap();
    assert_eq!(saved["tool_path"], tool.to_str().unwrap());
    assert_eq!(saved["task_id"], id);
    assert_eq!(p.next()["repair_brief"]["disposition"], "actionable");
}

#[test]
#[ignore = "requires existing OTP 28 via CODEGUARD_ERL_BIN"]
fn actual_path_otp_rechecks_candidate_task_then_repaired_source_without_flag() {
    let (p, id) = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_ERL_BIN").unwrap());
    let bin = p.0.join("real-bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(tool, bin.join("erl")).unwrap();
    let bad = p.verify_with_path(&id, bin.as_os_str(), None);
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["observation"], "still_blocked");
    assert_eq!(bad["native_scan"]["native"]["version"], "OTP 28");
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let good = p.verify_with_path(&id, bin.as_os_str(), None);
    assert_eq!(good["event_persisted"], true, "{good}");
    assert_eq!(good["observation"], "candidate_absent_unverified_policy");
    assert_eq!(
        good["native_scan"]["tool_path"],
        bad["native_scan"]["tool_path"]
    );
}

#[test]
fn erlang_native_diagnostics_guide_repair_and_preserve_stable_task_and_history() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let again = p.hook("file_changed", None, None);
    assert_eq!(
        again["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    let report = p.verify(&id, Some(&tool));
    assert_eq!(report["event_persisted"], true, "{report}");
    assert_eq!(report["observation"], "still_blocked");
    assert_eq!(report["native_scan"]["schema_version"], "0.2.0");
    assert_eq!(report["schema_version"], "0.13.0");
    let next = p.next();
    assert_eq!(next["schema_version"], "0.4.0", "{next}");
    let brief = &next["repair_brief"];
    assert_eq!(brief["action_id"], "repair-source");
    assert_eq!(
        brief["native_diagnostic_positions"][0]["rule_id"],
        "erlang.syntax.error"
    );
    assert!(brief["step"].as_str().unwrap().contains("Erlang"));
    let argv = brief["recheck_argv"].as_array().unwrap();
    assert_eq!(argv[argv.len() - 2], "--erl-tool");
    assert_eq!(argv[argv.len() - 1], tool.to_str().unwrap());
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let stale = p.next();
    assert_eq!(stale["repair_brief"]["native_confirmation_status"], "stale");
    assert_eq!(
        stale["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    let fixed = p.verify(&id, Some(&tool));
    assert_eq!(fixed["event_persisted"], true, "{fixed}");
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(tool, "changed").unwrap();
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "stale"
    );
}

#[test]
fn missing_erlang_tool_records_failed_observation_without_source_repair() {
    let (p, id) = Project::new();
    let r = p.verify(&id, None);
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "erlang_tool_not_found_on_path"
    );
    assert_eq!(
        p.next()["repair_brief"]["action_id"],
        "restore-checker-environment"
    );
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_reason"],
        "erlang_tool_not_found_on_path"
    );
}

#[test]
fn erlang_preprocessing_and_bad_native_protocol_remain_incomplete() {
    let (p, id) = Project::new();
    let tool=p.tool("/bin/cat >/dev/null\nprintf '%s' '{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":true,\"diagnostics_truncated\":false,\"diagnostics\":[{\"line\":2,\"column\":4,\"rule_id\":\"erlang.syntax.error\"}]}'");
    let r = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(r["observation"], "incomplete");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "erlang_preprocessing_unresolved"
    );
    assert_eq!(
        p.next()["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_reason"],
        "erlang_preprocessing_unresolved"
    );
    let tool=p.tool("/bin/cat >/dev/null\nprintf '%s' '{\"schema_version\":\"0.1.0\",\"schema_version\":\"0.1.0\"}'");
    let r = p.verify(&id, Some(&tool));
    assert_eq!(r["event_persisted"], true, "{r}");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "erlang_syntax_report_invalid"
    );
}

#[test]
fn repair_ready_uses_erlang_native_task_verification() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let r = p.hook("repair_ready", Some(&id), Some(&tool));
    assert_eq!(r["local_feedback"]["event_persisted"], true, "{r}");
    assert_eq!(r["local_feedback"]["observation"], "still_blocked");
    assert_eq!(r["schema_version"], "0.8.0");
    assert_eq!(
        r["local_feedback"]["native_confirmation_reason"],
        "erlang_native_syntax_diagnostics"
    );
    assert_eq!(
        r["local_feedback"]["native_diagnostic_positions"][0]["rule_id"],
        "erlang.syntax.error"
    );
    assert!(
        r["local_feedback"]["native_confirmation_ref"]["report_ref"]
            .as_str()
            .unwrap()
            .starts_with(".codeguard/reports/syntax-native-")
    );
    assert_eq!(r["delivery_decision"], "not_evaluated");
}

#[test]
fn erlang_human_feedback_uses_unicode_character_columns() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let out = p
        .command()
        .args(["task", "verify", &id])
        .arg(&p.0)
        .arg("--erl-tool")
        .arg(&tool)
        .arg("--format=human")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("字符列"), "{text}");
    assert!(!text.contains("字节列"), "{text}");
}

#[test]
fn mismatched_or_conflicting_language_tools_do_not_execute_or_acquire_lease() {
    let (p, id) = Project::new();
    let marker = p.0.join("tool-ran");
    let tool = p.tool(&format!("/usr/bin/touch '{}'", marker.display()));
    for options in [
        vec!["--zig-tool", tool.to_str().unwrap()],
        vec![
            "--erl-tool",
            tool.to_str().unwrap(),
            "--zig-tool",
            tool.to_str().unwrap(),
        ],
        vec!["--erl-tool", "relative"],
    ] {
        let out = p
            .command()
            .args(["task", "verify", &id])
            .arg(&p.0)
            .args(options)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!marker.exists());
        assert!(
            !p.0.join(format!(".codeguard/state/leases/{id}.json"))
                .exists()
        );
    }
}

#[test]
fn erlang_recheck_consumes_ready_attempt_and_preserves_caller_lease() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    p.verify(&id, Some(&tool));
    let lease = p.operation(&["task", "claim", &id, "--owner", "agent-a"]);
    let token = lease["lease_token"].as_str().unwrap();
    let attempt = p.operation(&[
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
    let aid = attempt["attempt_id"].as_str().unwrap();
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    p.operation(&[
        "task",
        "attempt",
        "finish",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--attempt-id",
        aid,
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "source_edit",
    ]);
    let report = p.operation(&[
        "task",
        "verify",
        &id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--erl-tool",
        tool.to_str().unwrap(),
    ]);
    assert_eq!(report["event_persisted"], true, "{report}");
    let run = report["native_scan"]["run_id"].as_str().unwrap();
    let event: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/events/verify-{run}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(event["attempt_id"], aid);
    assert_eq!(
        p.next()["repair_brief"]["history"]["awaiting_verification"],
        false
    );
    assert_eq!(
        p.operation(&[
            "task",
            "heartbeat",
            &id,
            "--owner",
            "agent-a",
            "--lease-token",
            token
        ])["command_status"],
        "complete"
    );
}

#[test]
fn erlang_history_import_rejects_forged_language_rule_version_and_positions() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let report = p.verify(&id, Some(&tool));
    let mut good = report["native_scan"].clone();
    good["run_id"] = json!("syntax-native-999-99");
    let path = p.0.join(".codeguard/reports/syntax-native-999-99.json");
    let sync = || {
        decode(
            &p.command()
                .args(["work", "sync"])
                .arg(&p.0)
                .arg("--format=json")
                .output()
                .unwrap(),
        )
    };
    fs::write(&path, serde_json::to_vec(&good).unwrap()).unwrap();
    assert_eq!(sync()["failed_reports"], 0);
    fs::remove_file(&path).unwrap();
    fs::remove_file(p.0.join(".codeguard/state/consumed/syntax-native-999-99.json")).unwrap();
    for (index, (pointer, value)) in [
        ("/schema_version", json!("9.0.0")),
        ("/target/language", json!("zig")),
        (
            "/native/diagnostics/0/rule_id",
            json!("zig.ast_check.error"),
        ),
        ("/native/diagnostics/0/column", json!(9999)),
        ("/native/preprocessing_unresolved", json!(true)),
        ("/native/diagnostics_truncated", json!(true)),
        ("/native/version", json!("OTP 29")),
        ("/native/reason", json!("ast_check_diagnostics")),
        ("/coverage_proven", json!(true)),
        ("/delivery_decision", json!("allow")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut bad = good.clone();
        bad["run_id"] = json!(format!("syntax-native-999-{}", 100 + index));
        *bad.pointer_mut(pointer).unwrap() = value;
        let path = p.0.join(format!(
            ".codeguard/reports/{}.json",
            bad["run_id"].as_str().unwrap()
        ));
        fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert_eq!(sync()["failed_reports"], 1, "{pointer}");
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn repeated_erlang_no_change_repairs_stop_with_concrete_decision() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    p.verify(&id, Some(&tool));
    let lease = p.operation(&["task", "claim", &id, "--owner", "agent-a"]);
    let token = lease["lease_token"].as_str().unwrap();
    for _ in 0..2 {
        let start = p.operation(&[
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
        p.operation(&[
            "task",
            "attempt",
            "finish",
            &id,
            "--owner",
            "agent-a",
            "--lease-token",
            token,
            "--attempt-id",
            start["attempt_id"].as_str().unwrap(),
            "--outcome",
            "no-change",
            "--note-code",
            "no_change",
        ]);
    }
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["disposition"], "needs_decision",
        "{next}"
    );
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 2);
    assert_eq!(
        next["repair_brief"]["native_confirmation_reason"],
        "erlang_native_syntax_diagnostics"
    );
    assert_eq!(next["repair_brief"]["history"]["budget"], 2);
}

#[test]
#[ignore = "需要显式 CODEGUARD_ERL_BIN 原生 OTP 28"]
fn real_otp_task_repair_and_missing_period_confirmation() {
    let tool = PathBuf::from(std::env::var("CODEGUARD_ERL_BIN").expect("explicit native erl"));
    let (p, id) = Project::new();
    let bad = p.verify(&id, Some(&tool));
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["observation"], "still_blocked");
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok\n").unwrap();
    let missing = p.verify(&id, Some(&tool));
    assert_eq!(missing["event_persisted"], true, "{missing}");
    assert_eq!(missing["observation"], "still_blocked");
    fs::write(p.0.join("app.erl"), "-module(app).\nf() -> ok.\n").unwrap();
    let fixed = p.verify(&id, Some(&tool));
    assert_eq!(fixed["event_persisted"], true, "{fixed}");
    assert_eq!(fixed["observation"], "candidate_absent_unverified_policy");
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "completed"
    );
}
