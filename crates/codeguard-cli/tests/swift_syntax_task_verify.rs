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
            "cg-swift-task-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.swift"), "func f(_ x: ) {}\n").unwrap();
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
            c.arg("--swift-tool").arg(t);
        }
        if let Some(path) = path {
            c.env("PATH", path);
        }
        c.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let req = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["app.swift"]} else {vec![]},"task_id":task,"write_outcome":"confirmed","host_claims_blocking":false}});
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
        let tool = self.0.join("swift-tool");
        fs::write(&tool, format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'Apple Swift version 6.4 (swiftlang-test)\\nTarget: test\\n'; exit 0; fi\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn verify(&self, id: &str, tool: Option<&Path>) -> Value {
        let mut c = self.command();
        c.args(["task", "verify", id])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(t) = tool {
            c.arg("--swift-tool").arg(t);
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
const PARSE_BY_SOURCE: &str = "input=$(/bin/cat)\ncase \"$input\" in *'x: )'*) printf '<stdin>:1:13: error: expected parameter type\\n' >&2; exit 1;; *) exit 0;; esac";

#[test]
fn hidden_swift_recovery_becomes_native_repair_then_stale_and_zero_diagnostic_observation() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let bad = p.verify(&id, Some(&tool));
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["native_scan"]["schema_version"], "0.4.0");
    assert_eq!(bad["schema_version"], "0.15.0");
    assert_eq!(bad["observation"], "still_blocked", "{bad}");
    let next = p.next();
    let b = &next["repair_brief"];
    assert_eq!(next["schema_version"], "0.6.0", "{next}");
    assert_eq!(b["action_id"], "repair-source");
    assert_eq!(b["native_column_unit"], "utf8_byte");
    assert_eq!(b["native_diagnostic_positions"][0]["column"], 13);
    assert_eq!(
        b["native_diagnostic_positions"][0]["rule_id"],
        "swift.parse.error"
    );
    assert!(b["step"].as_str().unwrap().contains("Swift"));
    assert_eq!(b["recheck_argv"][7], "--swift-tool");
    fs::write(p.0.join("app.swift"), "func f(_ x: Int) {}\n").unwrap();
    assert_eq!(
        p.next()["repair_brief"]["native_confirmation_status"],
        "stale"
    );
    assert_eq!(
        p.next()["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    let good = p.verify(&id, Some(&tool));
    assert_eq!(good["event_persisted"], true, "{good}");
    assert_eq!(good["observation"], "candidate_absent_unverified_policy");
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
fn swift_repair_ready_exposes_bounded_positions_not_raw_diagnostics() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let r = p.hook("repair_ready", Some(&id), Some(&tool));
    assert_eq!(r["schema_version"], "0.9.0", "{r}");
    assert_eq!(r["local_feedback"]["schema_version"], "0.3.0");
    assert_eq!(r["local_feedback"]["event_persisted"], true);
    assert_eq!(
        r["local_feedback"]["native_confirmation_reason"],
        "swift_native_parse_diagnostics"
    );
    assert_eq!(r["local_feedback"]["native_column_unit"], "utf8_byte");
    assert_eq!(
        r["local_feedback"]["native_diagnostic_positions"][0]["rule_id"],
        "swift.parse.error"
    );
    assert!(!r.to_string().contains("expected parameter type"));
    assert!(!r.to_string().contains(tool.to_str().unwrap()));
}

#[test]
fn swift_missing_and_malformed_or_inconsistent_native_reports_remain_incomplete() {
    let (p, id) = Project::new();
    let missing = p.verify(&id, None);
    assert_eq!(missing["event_persisted"], true, "{missing}");
    assert_eq!(
        missing["native_scan"]["native"]["reason"],
        "explicit_swift_tool_not_provided"
    );
    for body in [
        "/bin/cat >/dev/null; printf '<stdin>:1:999: error: wrong\\n' >&2; exit 1",
        "/bin/cat >/dev/null; printf 'driver crashed\\n' >&2; exit 1",
        "/bin/cat >/dev/null; printf '<stdin>:1:13: error: mismatch\\n' >&2; exit 0",
    ] {
        let tool = p.tool(body);
        let r = p.verify(&id, Some(&tool));
        assert_eq!(r["event_persisted"], true, "{r}");
        assert_eq!(r["observation"], "incomplete");
        assert_eq!(r["native_scan"]["native"]["diagnostics"], json!([]));
        assert_eq!(
            p.next()["repair_brief"]["action_id"],
            "restore-checker-environment"
        );
    }
}

#[test]
fn wrong_or_relative_swift_tool_is_rejected_before_lease_and_execution() {
    let (p, id) = Project::new();
    let marker = p.0.join("ran");
    let tool = p.tool(&format!("/usr/bin/touch '{}'", marker.display()));
    for flags in [
        vec!["--zig-tool", tool.to_str().unwrap()],
        vec!["--swift-tool", "relative"],
        vec![
            "--swift-tool",
            tool.to_str().unwrap(),
            "--erl-tool",
            tool.to_str().unwrap(),
        ],
    ] {
        let o = p
            .command()
            .args(["task", "verify", &id])
            .arg(&p.0)
            .args(flags)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2));
        assert!(!marker.exists());
        assert!(
            !p.0.join(format!(".codeguard/state/leases/{id}.json"))
                .exists()
        );
    }
}

#[test]
#[ignore = "requires existing Apple Swift 6.4 via CODEGUARD_SWIFTC_BIN"]
fn actual_swift_hidden_error_has_native_confirmation_and_fix_evidence() {
    let (p, id) = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_SWIFTC_BIN").unwrap());
    let bad = p.verify(&id, Some(&tool));
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["observation"], "still_blocked", "{bad}");
    assert_eq!(bad["native_scan"]["native"]["diagnostics"][0]["column"], 13);
    for (source, valid) in [
        ("let value = 1\n", true),
        ("func add(_ a: Int, _ b: Int) -> Int { a + b }\n", true),
        ("struct Point { let x: Int; let y: Int }\n", true),
        ("enum Status { case ready, failed(String) }\n", true),
        ("let name: String? = nil\n", true),
        ("let double = { (x: Int) in x * 2 }\n", true),
        ("let text = \"hello \\(1 + 2)\"\n", true),
        ("func fetch() async throws -> Int { return 1 }\n", true),
        ("func f() {\n", false),
        ("let s = \"oops\n", false),
        ("let x =\n", false),
        ("print(1, 2\n", false),
    ] {
        fs::write(p.0.join("app.swift"), source).unwrap();
        let r = p.verify(&id, Some(&tool));
        assert_eq!(r["event_persisted"], true, "{r}");
        assert_eq!(
            r["observation"],
            if valid {
                "candidate_absent_unverified_policy"
            } else {
                "still_blocked"
            },
            "{source}: {r}"
        );
    }
    fs::write(p.0.join("app.swift"), "func f(_ x: Int) {}\n").unwrap();
    let good = p.verify(&id, Some(&tool));
    assert_eq!(good["event_persisted"], true, "{good}");
    assert_eq!(good["observation"], "candidate_absent_unverified_policy");
}
#[test]
fn repeated_swift_no_change_repairs_stop_with_concrete_decision() {
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
        "swift_native_parse_diagnostics"
    );
    assert_eq!(next["repair_brief"]["history"]["budget"], 2);
}

#[test]
fn swift_history_import_rejects_forged_native_identity_and_byte_coordinates() {
    let (p, id) = Project::new();
    let tool = p.tool(PARSE_BY_SOURCE);
    let scan = p.verify(&id, Some(&tool))["native_scan"].clone();
    for (index, (pointer, value)) in [
        ("/schema_version", json!("0.1.0")),
        ("/target/language", json!("erlang")),
        ("/native/version", json!("Apple Swift 6.5")),
        (
            "/native/diagnostics/0/rule_id",
            json!("erlang.syntax.error"),
        ),
        ("/native/diagnostics/0/column", json!(999)),
        ("/native/reason", json!("swift_native_parse_no_diagnostics")),
        ("/coverage_proven", json!(true)),
        ("/delivery_decision", json!("allow")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut forged = scan.clone();
        let run = format!("syntax-native-999-{}", index + 100);
        forged["run_id"] = json!(run);
        *forged.pointer_mut(pointer).unwrap() = value;
        let path = p.0.join(format!(".codeguard/reports/{run}.json"));
        fs::write(&path, serde_json::to_vec(&forged).unwrap()).unwrap();
        let out = p
            .command()
            .args(["work", "sync"])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap();
        let r = decode(&out);
        assert_eq!(r["failed_reports"], 1, "{pointer}: {r}");
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn swift_version_timeout_and_multibyte_boundaries_never_become_source_diagnostics() {
    let (p, id) = Project::new();
    let tool = p.tool("/bin/cat >/dev/null; exit 0");
    fs::write(
        &tool,
        "#!/bin/sh\nprintf 'Apple Swift version 6.5 (test)\\n'\nexit 0\n",
    )
    .unwrap();
    let r = p.verify(&id, Some(&tool));
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "swift_version_unverified_or_unsupported"
    );
    fs::write(p.0.join("app.swift"), "let 界 = ;\n").unwrap();
    let tool = p.tool(
        "/bin/cat >/dev/null; printf '<stdin>:1:6: error: invalid byte boundary\\n' >&2; exit 1",
    );
    let r = p.verify(&id, Some(&tool));
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "swift_syntax_report_invalid"
    );
    let tool = p.tool(
        "/bin/cat >/dev/null; printf '<stdin>:1:11: error: expected initial value\\n' >&2; exit 1",
    );
    let r = p.verify(&id, Some(&tool));
    assert_eq!(r["observation"], "still_blocked", "{r}");
    let tool = p.tool("/bin/cat >/dev/null; /bin/sleep 2");
    let out = p
        .command()
        .args(["task", "verify", &id])
        .arg(&p.0)
        .arg("--swift-tool")
        .arg(&tool)
        .args(["--timeout", "250ms", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let r = decode(&out);
    assert_eq!(r["observation"], "incomplete", "{r}");
    assert_eq!(
        r["native_scan"]["native"]["reason"],
        "request_deadline_exceeded"
    );
    assert_eq!(r["native_scan"]["native"]["diagnostics"], json!([]));
}
