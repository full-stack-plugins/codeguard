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
        Self::new_with_source("fun f(x: ) = x\n")
    }
    fn new_with_source(source: &str) -> (Self, String) {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-kotlin-task-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("App.kt"), source).unwrap();
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
            c.arg("--kotlinc-tool").arg(t);
        }
        if let Some(path) = path {
            c.env("PATH", path);
        }
        c.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let req = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":event,"changed_paths":if event=="file_changed" {vec!["App.kt"]} else {vec![]},"task_id":task,"write_outcome":"confirmed","host_claims_blocking":false}});
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
        let tool = self.0.join("kotlinc");
        fs::write(&tool, format!("#!/bin/sh\nif [ \"$1\" = -version ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn verify(&self, id: &str, tool: Option<&Path>) -> Value {
        let mut c = self.command();
        c.args(["task", "verify", id])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(t) = tool {
            c.arg("--kotlinc-tool").arg(t);
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
}
fn decode(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}

const BY_SOURCE: &str = "input=$(/bin/cat \"$1\")\ncase \"$input\" in *'x: )'*) printf '%s:1:10: error: [SYNTAX] Type expected.\\nfun f(x: ) = x\\n         ^\\n' \"$1\" >&2; exit 1;; *) exit 0;; esac";
#[test]
fn kotlin_stable_task_native_recheck_and_hook_keep_evidence_current() {
    let (p, id) = Project::new();
    let first = p.next();
    assert_eq!(
        first["repair_brief"]["native_adapter"]["language"], "kotlin",
        "{first}"
    );
    assert_eq!(
        first["repair_brief"]["disposition"],
        "verification_required"
    );
    let show = decode(
        &p.command()
            .args(["task", "show", &id])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap(),
    );
    assert_eq!(
        show["next_actions"],
        json!([first["repair_brief"]["recheck_argv"]])
    );
    let tool = p.tool(BY_SOURCE);
    let bad = p.verify(&id, Some(&tool));
    assert_eq!(bad["event_persisted"], true, "{bad}");
    assert_eq!(bad["observation"], "still_blocked", "{bad}");
    assert_eq!(bad["native_scan"]["schema_version"], "0.5.0");
    let next = p.next();
    let brief = &next["repair_brief"];
    assert_eq!(brief["task_id"], id);
    assert_eq!(brief["action_id"], "repair-source", "{next}");
    assert_eq!(
        brief["native_diagnostic_positions"][0]["rule_id"],
        "kotlin.syntax"
    );
    assert_eq!(brief["native_diagnostic_positions"][0]["column_byte"], 10);
    assert!(brief["recheck_argv"].to_string().contains("--kotlinc-tool"));
    fs::write(p.0.join("App.kt"), "class C {}\n").unwrap();
    let stale = p.next();
    assert_eq!(stale["repair_brief"]["native_confirmation_status"], "stale");
    assert_eq!(
        stale["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    let fixed = p.verify(&id, Some(&tool));
    assert_eq!(
        fixed["observation"], "candidate_absent_unverified_policy",
        "{fixed}"
    );
    assert_eq!(fixed["event_persisted"], true, "{fixed}");
    let clean = p.next();
    assert_eq!(clean["repair_brief"]["task_id"], id);
    assert_eq!(
        clean["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    let hook = p.hook("repair_ready", Some(&id), Some(&tool));
    assert_eq!(
        hook["local_feedback"]["native_confirmation_status"], "completed",
        "{hook}"
    );
    assert_eq!(hook["local_feedback"]["event_persisted"], true);
    assert_eq!(
        hook["local_feedback"]["native_diagnostic_positions"],
        json!([])
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn kotlin_context_and_tool_failures_do_not_authorize_syntax_repairs() {
    let (p, id) = Project::new();
    fs::write(p.0.join("App.kt"), "fun f(x: Missing) = x\n").unwrap();
    let tool=p.tool("printf '%s:1:10: error: [UNRESOLVED_REFERENCE] Missing.\\nfun f(x: Missing) = x\\n         ^\\n' \"$1\" >&2; exit 1");
    let context = p.verify(&id, Some(&tool));
    assert_eq!(context["event_persisted"], true, "{context}");
    assert_eq!(context["observation"], "incomplete");
    let next = p.next();
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"],
        json!([])
    );
    assert_ne!(next["repair_brief"]["action_id"], "repair-source");
    assert_eq!(
        context["native_scan"]["native"]["context_diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let missing = p.verify(&id, None);
    assert_eq!(missing["event_persisted"], true, "{missing}");
    assert_eq!(
        missing["native_scan"]["native"]["reason"],
        "kotlin_tool_not_found"
    );
    assert!(
        p.next()["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("Kotlin")
    );
}

#[test]
fn mixed_kotlin_syntax_and_context_preserve_actionable_source_evidence() {
    let (p, id) = Project::new();
    let tool=p.tool("printf '%s:1:10: error: [SYNTAX] Type expected.\\nfun f(x: ) = x\\n         ^\\n%s:1:13: error: [UNRESOLVED_REFERENCE] Missing.\\nfun f(x: ) = x\\n            ^\\n' \"$1\" \"$1\" >&2; exit 1");
    let mixed = p.verify(&id, Some(&tool));
    assert_eq!(mixed["event_persisted"], true, "{mixed}");
    assert_eq!(mixed["observation"], "still_blocked", "{mixed}");
    let next = p.next();
    assert_eq!(next["repair_brief"]["action_id"], "repair-source", "{next}");
    assert_eq!(
        next["repair_brief"]["native_diagnostic_positions"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "{next}"
    );
    assert_eq!(
        next["repair_brief"]["native_context_diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "{next}"
    );
    let hook = p.hook("repair_ready", Some(&id), Some(&tool));
    assert_eq!(
        hook["local_feedback"]["observation"], "still_blocked",
        "{hook}"
    );
    assert_eq!(
        hook["local_feedback"]["native_diagnostic_positions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        hook["local_feedback"]["native_context_diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn same_source_native_zero_is_grammar_counterevidence_not_a_source_fix() {
    let source = "object C { val value = 1 }\n";
    let (p, id) = Project::new_with_source(source);
    let tool = p.tool("exit 0");
    let verified = p.verify(&id, Some(&tool));
    assert_eq!(
        verified["observation"],
        "candidate_absent_unverified_policy"
    );
    let next = p.next();
    let brief = &next["repair_brief"];
    let step = brief["step"].as_str().unwrap();
    assert!(
        step.contains("同一源码") && step.contains("grammar反证候选"),
        "{next}"
    );
    assert_eq!(brief["task_id"], id);
    assert_eq!(brief["disposition"], "verification_required");
    assert_eq!(fs::read_to_string(p.0.join("App.kt")).unwrap(), source);
    assert_eq!(next["delivery_decision"], "not_evaluated");
    fs::write(p.0.join("App.kt"), "class C {}\n").unwrap();
    let stale = p.next();
    assert!(
        !stale["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("grammar反证候选"),
        "{stale}"
    );
    p.verify(&id, Some(&tool));
    let changed = p.next();
    assert!(
        !changed["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("grammar反证候选"),
        "{changed}"
    );
}

#[test]
#[ignore = "需要显式已安装Kotlin/JVM2.4.10；只验证局部同字节反证，不批准关闭"]
fn real_kotlin_same_source_counterevidence_preserves_legal_source_and_open_task() {
    let tool = std::env::var_os("CODEGUARD_KOTLINC_BIN").expect("指定已安装的kotlinc绝对路径");
    let tool = PathBuf::from(tool).canonicalize().unwrap();
    let source = "object C { val value = 1 }\n";
    let (p, id) = Project::new_with_source(source);
    let mut command = p.command();
    // 保留实际已安装JDK/bash的环境；首次Hook仍隔离PATH，固定WASM来源。
    command.env("PATH", std::env::var_os("PATH").unwrap());
    let out = command
        .args(["task", "verify", &id])
        .arg(&p.0)
        .args(["--format=json", "--timeout", "90s", "--kotlinc-tool"])
        .arg(&tool)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = decode(&out);
    assert_eq!(
        report["observation"], "candidate_absent_unverified_policy",
        "{report}"
    );
    assert_eq!(
        report["native_scan"]["native"]["status"], "completed",
        "{report}"
    );
    let next = p.next();
    let step = next["repair_brief"]["step"].as_str().unwrap();
    assert!(
        step.contains("同一源码") && step.contains("grammar反证候选"),
        "{next}"
    );
    assert_eq!(fs::read_to_string(p.0.join("App.kt")).unwrap(), source);
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(
        std::env::temp_dir().join("codeguard-kotlin-counterevidence-native-report.json"),
        serde_json::to_vec_pretty(&json!({"verification":report,"next":next})).unwrap(),
    )
    .unwrap();
}

#[test]
fn archived_real_counterevidence_binds_original_bytes_without_delivery_approval() {
    use sha2::{Digest, Sha256};
    let archive = codeguard_adapters::parse_unique_json(include_bytes!(
        "../../../tests/acceptance/evidence/kotlin-same-source-counterevidence-2026-10-06.json"
    ))
    .unwrap();
    let native = &archive["verification"]["native_scan"];
    let source_sha = format!("{:x}", Sha256::digest(b"object C { val value = 1 }\n"));
    assert_eq!(native["target"]["source_sha256"], source_sha);
    assert_eq!(native["original_report"]["source_sha256"], source_sha);
    assert_eq!(native["native"]["version"], "kotlinc-jvm 2.4.10");
    assert_eq!(native["native"]["status"], "completed");
    assert_eq!(native["native"]["tool_identity_scope"], "launcher_only");
    assert_eq!(native["native"]["diagnostics"], json!([]));
    assert_eq!(native["native"]["context_diagnostics"], json!([]));
    assert_eq!(native["authority"], "local_unverified");
    assert_eq!(native["coverage_proven"], false);
    assert_eq!(native["delivery_decision"], "not_evaluated");
    assert_eq!(archive["next"]["delivery_decision"], "not_evaluated");
    assert_eq!(
        archive["next"]["repair_brief"]["native_confirmation_ref"]["run_id"],
        native["run_id"]
    );
    assert_eq!(
        archive["next"]["repair_brief"]["task_id"],
        native["task_id"]
    );
    assert!(
        archive["next"]["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("grammar反证候选")
    );
}
