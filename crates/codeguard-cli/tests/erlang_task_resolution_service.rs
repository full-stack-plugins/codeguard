#![cfg(all(unix, feature = "wasm-precheck"))]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, ErlangTaskResolutionRequest,
    verify_erlang_task_resolution,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const BAD: &[u8] = b"-module(demo).\nf( -> ok.\n";
const GOOD: &[u8] = b"-module(demo).\nf() -> ok.\n";
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
const NORMAL_TOOL: &str = "input=$(/bin/cat)\ncase \"$input\" in *'f( ->'*) diagnostics='[{\"line\":2,\"column\":4,\"rule_id\":\"erlang.syntax.error\"}]';; *) diagnostics='[]';; esac\nprintf '{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":%s}' \"$diagnostics\"";
fn write_tool(root: &Path, body: &str) -> PathBuf {
    let p = root.join("erl-tool");
    fs::write(
        &p,
        format!(
            "#!/bin/sh\ncase \"$*\" in *system_info*) printf 'OTP 28\\n'; exit 0;; esac\n{body}\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    p
}
struct Project {
    root: PathBuf,
    id: String,
    workspace: String,
    original_report: String,
    grammar: Option<String>,
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Project {
    fn new() -> Self {
        Self::create(false)
    }
    fn native_first() -> Self {
        Self::create(true)
    }
    fn create(native_first: bool) -> Self {
        Self::create_with_tool(native_first, None)
    }
    fn create_with_tool(native_first: bool, native_tool: Option<&Path>) -> Self {
        let host_size = fs::metadata(std::env::current_exe().unwrap())
            .unwrap()
            .len();
        assert!(
            host_size <= 256 * 1024 * 1024,
            "SDK test host exceeds the product artifact budget: {host_size} bytes; use CARGO_PROFILE_TEST_DEBUG=0 without widening the product limit"
        );
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-erlang-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.erl"), BAD).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let id = if native_first {
            let tool = native_tool
                .map(Path::to_path_buf)
                .unwrap_or_else(|| write_tool(&root, NORMAL_TOOL));
            let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["lint", "erlang"])
                .arg(root.join("app.erl"))
                .arg("--erl-tool")
                .arg(tool)
                .arg("--format=json")
                .output()
                .unwrap();
            assert_eq!(o.status.code(), Some(3));
            let r: Value = serde_json::from_slice(&o.stdout).unwrap();
            assert_eq!(r["native"]["status"], "diagnostics_observed");
            assert!(r["syntax_precheck"].is_null());
            r["task_id"].as_str().unwrap().to_owned()
        } else {
            let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["hook", "execute"])
                .arg(&root)
                .args(["--format=json", "--timeout", "30s"])
                .env("PATH", &root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
            let o = child.wait_with_output().unwrap();
            let r: Value = serde_json::from_slice(&o.stdout).unwrap();
            r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        let original: Value = serde_json::from_slice(
            &fs::read(root.join(format!(
                ".codeguard/reports/{}.json",
                fact["first_run_id"].as_str().unwrap()
            )))
            .unwrap(),
        )
        .unwrap();
        Self {
            root,
            id,
            workspace: fact["workspace_id"].as_str().unwrap().into(),
            original_report: fact["first_report_sha256"].as_str().unwrap().into(),
            grammar: original["observations"][0]["grammar_sha256"]
                .as_str()
                .map(str::to_owned),
        }
    }
    fn tool(&self, body: &str) -> PathBuf {
        write_tool(&self.root, body)
    }
    fn normal_tool(&self) -> PathBuf {
        self.tool(NORMAL_TOOL)
    }
    fn policy(&self, tool: &Path) -> Value {
        json!({"schema_version":"1.1.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"syntax.native_confirmation","scope":"app.erl"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":self.grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"erlang.syntax.error","native_version":"OTP 28"})
    }
    fn verify(&self, tool: &Path, policy: &Value) -> Result<Value, &'static str> {
        self.verify_borrowed(tool, policy, None)
    }
    fn verify_borrowed(
        &self,
        tool: &Path,
        policy: &Value,
        borrowed: Option<(&str, &str)>,
    ) -> Result<Value, &'static str> {
        self.verify_raw(
            tool,
            &serde_json::to_vec(policy).unwrap(),
            borrowed,
            "valid",
        )
    }
    fn verify_raw(
        &self,
        tool: &Path,
        raw: &[u8],
        borrowed: Option<(&str, &str)>,
        host_case: &str,
    ) -> Result<Value, &'static str> {
        let pair = Ed25519KeyPair::from_seed_unchecked(&[9; 32]).unwrap();
        let payload=json!({"schema_version":"1.0","workspace_id":self.workspace,"policy_revision":"p1","baseline_commit":"a".repeat(40),"revision_sequence":1,"issued_at":100,"expires_at":200,"snapshot_sha256":sha(raw)}).to_string();
        let mut message = b"codeguard.approval.v1\0fixture-host\0".to_vec();
        message.extend_from_slice(payload.as_bytes());
        let signature = pair
            .sign(&message)
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let envelope=json!({"schema_version":"1.0","key_id":"fixture-host","approval_json":payload,"signature_hex":signature}).to_string();
        let trust = ApprovalTrustKey {
            key_id: "fixture-host".into(),
            public_key: if host_case == "wrong_key" {
                [0; 32]
            } else {
                pair.public_key().as_ref().try_into().unwrap()
            },
            valid_from: 1,
            valid_until: 300,
            revoked: host_case == "revoked",
        };
        let context = ApprovalVerificationContext {
            workspace_id: &self.workspace,
            policy_revision: "p1",
            baseline_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            now_unix: if host_case == "clock_missing" {
                None
            } else if host_case == "expired" {
                Some(201)
            } else {
                Some(150)
            },
            minimum_sequence: if host_case == "rollback" { 2 } else { 1 },
            max_lifetime_seconds: 100,
        };
        verify_erlang_task_resolution(&ErlangTaskResolutionRequest {
            root: &self.root,
            task_id: &self.id,
            tool,
            original_source: BAD,
            policy_bytes: raw,
            envelope_bytes: envelope.as_bytes(),
            trust: &trust,
            context: &context,
            deadline: Instant::now() + Duration::from_secs(30),
            borrowed_lease: borrowed,
        })
    }
    fn events(&self) -> Vec<Value> {
        let dir = self
            .root
            .join(format!(".codeguard/findings/{}/events", self.id));
        let mut v = Vec::new();
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("lifecycle-")
            {
                v.push(serde_json::from_slice(&fs::read(p).unwrap()).unwrap());
            }
        }
        v
    }
}
#[test]
fn approved_erlang_resolution_closes_replays_and_reopens_the_same_task() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["state"], "resolved");
    assert_eq!(receipt["outcome"], "code_fixed");
    assert_eq!(receipt["delivery_decision"], "not_evaluated");
    assert_eq!(p.events().len(), 2);
    export(&p, &receipt, &policy, "fixture-resolved");
    let repeated = p.verify(&tool, &policy).unwrap();
    assert_eq!(repeated["event_ref"], receipt["event_ref"]);
    assert_eq!(p.events().len(), 2);
    fs::write(p.root.join("app.erl"), BAD).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--erl-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let recheck: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(recheck["event_persisted"], true);
    assert_eq!(p.events().len(), 3);
    assert!(
        p.events()
            .iter()
            .any(|event| event["event"]["kind"]["event"] == "reopened")
    );
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
    assert_eq!(p.events().len(), 3);
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "resolved");
    assert_eq!(p.events().len(), 4);
    let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
    assert_eq!(brief["disposition"], "verification_required");
}

fn export(p: &Project, receipt: &Value, policy: &Value, label: &str) {
    if let Ok(dir) = std::env::var("CODEGUARD_ERLANG_RESOLUTION_REPORT_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        let prefix = format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        for (suffix, value) in [
            ("policy", policy.clone()),
            ("receipt", receipt.clone()),
            (
                "evidence",
                serde_json::from_slice(
                    &fs::read(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
                )
                .unwrap(),
            ),
        ] {
            fs::write(
                dir.join(format!("{prefix}-{suffix}.json")),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
        }
        for (i, event) in p.events().iter().enumerate() {
            fs::write(
                dir.join(format!("{prefix}-event-{i}.json")),
                serde_json::to_vec_pretty(event).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn erlang_original_native_counterexample_requires_false_positive_review() {
    let p = Project::new();
    let tool = p.tool("/bin/cat >/dev/null\nprintf '%s' '{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":[]}'");
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["outcome"], "false_positive_review_required");
    assert_ne!(receipt["state"], "resolved");
    export(&p, &receipt, &policy, "fixture-counterexample");
    let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
    assert!(brief["step"].as_str().unwrap().contains("误报"));
    assert!(brief["step"].as_str().unwrap().contains("首次 WASM"));
    assert!(
        brief["step"]
            .as_str()
            .unwrap()
            .contains("不直接认定 grammar 缺陷")
    );
}

#[test]
fn erlang_macro_empty_truncated_and_failed_native_results_cannot_close() {
    for (label, body) in [
        (
            "macro",
            "{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":true,\"diagnostics_truncated\":false,\"diagnostics\":[]}",
        ),
        (
            "empty",
            "{\"schema_version\":\"0.1.0\",\"forms\":0,\"preprocessing\":false,\"diagnostics_truncated\":false,\"diagnostics\":[]}",
        ),
        (
            "truncated",
            "{\"schema_version\":\"0.1.0\",\"forms\":2,\"preprocessing\":false,\"diagnostics_truncated\":true,\"diagnostics\":[]}",
        ),
        ("failed", "not json"),
    ] {
        let p = Project::new();
        let tool = p.tool(&format!("/bin/cat >/dev/null\nprintf '%s' '{body}'"));
        let policy = p.policy(&tool);
        fs::write(p.root.join("app.erl"), GOOD).unwrap();
        let receipt = p.verify(&tool, &policy).unwrap();
        assert_eq!(
            receipt["outcome"], "native_incomplete",
            "{label}: {receipt}"
        );
        assert_ne!(receipt["state"], "resolved");
        export(&p, &receipt, &policy, &format!("fixture-{label}"));
    }
}

#[test]
fn erlang_wrong_language_policy_and_untrusted_host_do_not_launch_native_tool() {
    let p = Project::new();
    let marker = p.root.join("native-started");
    let tool = p.tool(&format!("touch '{}'", marker.display()));
    let policy = p.policy(&tool);
    for (field, value) in [
        ("schema_version", "1.0.0"),
        ("native_version", "0.16.0"),
        ("native_rule_id", "zig.ast_check.error"),
        ("grammar_sha256", &"b".repeat(64)),
    ] {
        let mut bad = policy.clone();
        bad[field] = json!(value);
        assert!(p.verify(&tool, &bad).is_err(), "{field}");
        assert!(!marker.exists());
    }
    let raw = serde_json::to_vec(&policy).unwrap();
    for host in [
        "wrong_key",
        "revoked",
        "clock_missing",
        "expired",
        "rollback",
    ] {
        assert!(p.verify_raw(&tool, &raw, None, host).is_err(), "{host}");
        assert!(!marker.exists());
    }
}

#[test]
fn erlang_source_mutation_during_original_replay_cannot_close() {
    let p = Project::new();
    let tool = p.normal_tool();
    let script = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        script
            + &format!(
                "\nprintf -- '-module(demo).\\nf() -> ok.\\n%% concurrent edit\\n' > '{}'\n",
                p.root.join("app.erl").display()
            ),
    )
    .unwrap();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["outcome"], "inputs_stale");
    assert_ne!(receipt["state"], "resolved");
    export(&p, &receipt, &policy, "fixture-mutated");
}

#[test]
#[ignore = "需要本机已安装 OTP 28；宿主签名与信任根仍是独立测试夹具"]
fn real_otp28_original_fixed_and_recurrent_source_resolution() {
    let tool = PathBuf::from(std::env::var("CODEGUARD_ERL_BIN").unwrap());
    for native_first in [false, true] {
        let p = Project::create_with_tool(native_first, Some(&tool));
        let policy = p.policy(&tool);
        fs::write(p.root.join("app.erl"), GOOD).unwrap();
        let receipt = p.verify(&tool, &policy).unwrap();
        assert_eq!(receipt["state"], "resolved");
        assert_eq!(receipt["outcome"], "code_fixed");
        export(
            &p,
            &receipt,
            &policy,
            if native_first {
                "otp28-native-first-resolved"
            } else {
                "otp28-wasm-first-resolved"
            },
        );
        fs::write(p.root.join("app.erl"), BAD).unwrap();
        let receipt = p.verify(&tool, &policy).unwrap();
        assert_eq!(receipt["state"], "open");
        assert_eq!(receipt["outcome"], "still_present");
        export(
            &p,
            &receipt,
            &policy,
            if native_first {
                "otp28-native-first-reopened"
            } else {
                "otp28-wasm-first-reopened"
            },
        );
    }
}
#[test]
fn erlang_resolution_consumes_attempt_without_releasing_borrowed_lease() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    let command = |args: &[&str]| -> Value {
        let split = if args[1] == "attempt" { 4 } else { 3 };
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(&args[..split])
            .arg(&p.root)
            .args(&args[split..])
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(if args[1] == "verify" { 3 } else { 0 }),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    };
    let tool_arg = tool.to_str().unwrap();
    command(&["task", "verify", &p.id, "--erl-tool", tool_arg]);
    let n = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let n: Value = serde_json::from_slice(&n.stdout).unwrap();
    let action = n["repair_brief"]["action_id"].as_str().unwrap();
    let claim = command(&["task", "claim", &p.id, "--owner", "agent-a"]);
    let token = claim["lease_token"].as_str().unwrap();
    let start = command(&[
        "task",
        "attempt",
        "start",
        &p.id,
        "--owner",
        "agent-a",
        "--lease-token",
        token,
        "--action-id",
        action,
    ]);
    let attempt = start["attempt_id"].as_str().unwrap();
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    command(&[
        "task",
        "attempt",
        "finish",
        &p.id,
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
    let r = p
        .verify_borrowed(&tool, &policy, Some(("agent-a", token)))
        .unwrap();
    assert_eq!(r["state"], "resolved");
    let n = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let n: Value = serde_json::from_slice(&n.stdout).unwrap();
    assert_eq!(
        n["repair_brief"]["history"]["awaiting_verification"], false,
        "{n}"
    );
    let lease: Value = serde_json::from_slice(
        &fs::read(
            p.root
                .join(format!(".codeguard/state/leases/{}.json", p.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(lease["status"], "active");
}

#[test]
fn native_first_erlang_resolution_preserves_null_grammar_and_reopens() {
    let p = Project::native_first();
    assert!(p.grammar.is_none());
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    assert!(policy["grammar_sha256"].is_null());
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["state"], "resolved");
    assert_eq!(receipt["outcome"], "code_fixed");
    export(&p, &receipt, &policy, "native-first-resolved");
    let evidence: Value = serde_json::from_slice(
        &fs::read(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert!(evidence["grammar_sha256"].is_null());
    fs::write(p.root.join("app.erl"), BAD).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--erl-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
}

#[test]
fn rehashed_cross_language_resolution_history_is_rejected() {
    let p = Project::new();
    let marker = p.root.join("native-calls");
    let tool = p.tool(&format!(
        "printf x >> '{}'\n{NORMAL_TOOL}",
        marker.display()
    ));
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.erl"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    let mut evidence: Value = serde_json::from_slice(
        &fs::read(p.root.join(r["evidence_ref"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    evidence["schema_version"] = json!("0.1.0");
    for key in ["original_native", "current_native"] {
        let n = &mut evidence[key];
        n["version"] = json!("0.16.0");
        n["reason"] = json!(if key == "original_native" {
            "ast_check_diagnostics"
        } else {
            "ast_check_no_diagnostics"
        });
        n["diagnostic_count"] = json!(n["diagnostics"].as_array().unwrap().len());
        for d in n["diagnostics"].as_array_mut().unwrap() {
            d["rule_id"] = json!("zig.ast_check.error");
        }
        n.as_object_mut()
            .unwrap()
            .remove("preprocessing_unresolved");
        n.as_object_mut().unwrap().remove("diagnostics_truncated");
    }
    let bytes = serde_json::to_vec_pretty(&evidence).unwrap();
    let evidence_sha = sha(&bytes);
    fs::write(
        p.root.join(format!(
            ".codeguard/state/resolution_evidence/{evidence_sha}.json"
        )),
        bytes,
    )
    .unwrap();
    let event_path = p.root.join(r["event_ref"].as_str().unwrap());
    let mut record: codeguard_core::TaskLifecycleRecord =
        serde_json::from_slice(&fs::read(&event_path).unwrap()).unwrap();
    record.evidence_sha256 = Some(evidence_sha.clone());
    record.event.kind = codeguard_core::TaskLifecycleKind::Resolved {
        cause: codeguard_core::ResolutionCause::CodeFixed,
        evidence_sha256: evidence_sha,
    };
    record.event.event_id.clear();
    record.event.event_id = format!("event-{}", sha(&serde_json::to_vec(&record).unwrap()));
    let replacement = event_path
        .parent()
        .unwrap()
        .join(format!("lifecycle-{}.json", record.event.event_id));
    fs::remove_file(event_path).unwrap();
    fs::write(replacement, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let calls = fs::read(&marker).unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_evidence_binding_invalid")
    );
    assert_eq!(
        fs::read(&marker).unwrap(),
        calls,
        "invalid language history must fail before native execution"
    );
    let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
    assert_eq!(brief["disposition"], "needs_decision");
}

#[test]
fn native_first_policy_cannot_invent_a_grammar_or_replace_the_original_tool() {
    let p = Project::native_first();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    let mut wrong = policy.clone();
    wrong["grammar_sha256"] = json!("a".repeat(64));
    assert_eq!(
        p.verify(&tool, &wrong),
        Err("task_resolution_original_identity_mismatch")
    );
    wrong.as_object_mut().unwrap().remove("grammar_sha256");
    assert_eq!(
        p.verify(&tool, &wrong),
        Err("task_resolution_policy_invalid")
    );
    let changed = p.tool("/bin/cat >/dev/null; exit 1");
    assert_eq!(
        p.verify(&changed, &p.policy(&changed)),
        Err("task_resolution_original_identity_mismatch")
    );
    assert!(p.events().is_empty());
}
