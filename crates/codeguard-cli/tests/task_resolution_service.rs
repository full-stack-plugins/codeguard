#![cfg(all(unix, feature = "wasm-precheck"))]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, ZigTaskResolutionRequest,
    verify_zig_task_resolution,
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
const BAD: &[u8] = b"pub fn main( void {\n";
const GOOD: &[u8] = b"pub fn main() void {}\n";
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
struct Project {
    root: PathBuf,
    id: String,
    workspace: String,
    original_report: String,
    grammar: String,
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Project {
    fn new() -> Self {
        let host_size = fs::metadata(std::env::current_exe().unwrap())
            .unwrap()
            .len();
        assert!(
            host_size <= 256 * 1024 * 1024,
            "SDK test host exceeds the product artifact budget: {host_size} bytes; use CARGO_PROFILE_TEST_DEBUG=0 without widening the product limit"
        );
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), BAD).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["hook", "execute"])
            .arg(&root)
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", &root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.zig"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
        let o = child.wait_with_output().unwrap();
        let r: Value = serde_json::from_slice(&o.stdout).unwrap();
        let id = r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
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
                .unwrap()
                .into(),
        }
    }
    fn tool(&self, body: &str) -> PathBuf {
        let p = self.root.join("zig-tool");
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
    fn normal_tool(&self) -> PathBuf {
        self.tool("input=$(/bin/cat)\ncase \"$input\" in *'main( void'*) printf '<stdin>:1:13: error: expected closing token\\n' >&2; exit 1;; *) exit 0;; esac")
    }
    fn policy(&self, tool: &Path) -> Value {
        json!({"schema_version":"1.0.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"syntax.native_confirmation","scope":"app.zig"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":self.grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"zig.ast_check.error","native_version":"0.16.0"})
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
        verify_zig_task_resolution(&ZigTaskResolutionRequest {
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
fn approved_native_recheck_closes_replays_and_reopens_the_same_task() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    assert_eq!(r["outcome"], "code_fixed");
    export_artifacts(&p, &r, &policy, "resolved");
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert_eq!(p.events().len(), 2);
    let repeated = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["event_ref"], repeated["event_ref"]);
    assert_eq!(p.events().len(), 2);
    fs::write(p.root.join("app.zig"), BAD).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--zig-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let local: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(local["observation"], "still_blocked");
    assert_eq!(local["event_persisted"], true);
    assert_eq!(
        p.events().len(),
        3,
        "ordinary task verify must record recurrence on the same parent chain"
    );
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "open");
    assert_eq!(r["outcome"], "still_present");
    export_artifacts(&p, &r, &policy, "reopened");
    assert_eq!(p.events().len(), 3);
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "resolved");
    assert_eq!(p.events().len(), 4);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            p.root
                .join(format!(".codeguard/findings/{}/finding.json", p.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next"])
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let n: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(n["delivery_decision"], "not_evaluated");
    assert_eq!(n["repair_brief"]["disposition"], "verification_required");
}
#[test]
fn native_counterexample_and_environment_failures_do_not_close() {
    let p = Project::new();
    let tool = p.tool("/bin/cat >/dev/null\nexit 0");
    let r = p.verify(&tool, &p.policy(&tool)).unwrap();
    assert_eq!(r["state"], "verification_required");
    assert_eq!(r["outcome"], "false_positive_review_required");
    export_artifacts(&p, &r, &p.policy(&tool), "false-positive");
    let tool = p.tool("/bin/cat >/dev/null\nexit 2");
    let r = p.verify(&tool, &p.policy(&tool)).unwrap();
    assert_eq!(r["state"], "verification_required");
    assert_eq!(r["outcome"], "native_incomplete");
    export_artifacts(&p, &r, &p.policy(&tool), "incomplete");
    assert!(
        !p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "resolved")
    );
}
#[test]
fn unavailable_tool_during_recheck_produces_readable_failure_evidence() {
    let p = Project::new();
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let tool = p.tool("/bin/cat >/dev/null\n/bin/rm -- \"$0\"\nexit 2");
    let policy = p.policy(&tool);
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "verification_required");
    export_artifacts(&p, &r, &policy, "tool-missing");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let n: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        n["repair_brief"]["disposition"], "verification_required",
        "{n}"
    );
}

#[test]
fn signed_identity_and_purpose_changes_are_rejected_before_native_execution() {
    let p = Project::new();
    let marker = p.root.join("executed");
    let tool = p.tool(&format!("/usr/bin/touch '{}'\nexit 0", marker.display()));
    for field in [
        "original_source_sha256",
        "original_report_sha256",
        "grammar_sha256",
        "tool_sha256",
        "adapter_sha256",
    ] {
        let mut policy = p.policy(&tool);
        policy[field] = json!("0".repeat(64));
        assert!(p.verify(&tool, &policy).is_err(), "{field}");
    }
    let mut policy = p.policy(&tool);
    policy["identity"]["task_id"] = json!("CG-B-00000000000000000000000000000000");
    assert!(p.verify(&tool, &policy).is_err());
    let mut policy = p.policy(&tool);
    policy["delivery_decision"] = json!("allow");
    assert!(p.verify(&tool, &policy).is_err());
    assert!(!marker.exists());
    assert!(p.events().is_empty());
}
#[test]
fn forked_history_and_missing_evidence_require_reconciliation() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    let event_path = p.root.join(r["event_ref"].as_str().unwrap());
    let original_event_bytes = fs::read(&event_path).unwrap();
    let mut wrong_cause: codeguard_core::TaskLifecycleRecord =
        serde_json::from_slice(&original_event_bytes).unwrap();
    if let codeguard_core::TaskLifecycleKind::Resolved { cause, .. } = &mut wrong_cause.event.kind {
        *cause = codeguard_core::ResolutionCause::PolicyResolved;
    }
    wrong_cause.event.event_id.clear();
    wrong_cause.event.event_id =
        format!("event-{}", sha(&serde_json::to_vec(&wrong_cause).unwrap()));
    let wrong_path = event_path
        .parent()
        .unwrap()
        .join(format!("lifecycle-{}.json", wrong_cause.event.event_id));
    fs::remove_file(&event_path).unwrap();
    fs::write(
        &wrong_path,
        serde_json::to_vec_pretty(&wrong_cause).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_evidence_binding_invalid")
    ));
    fs::remove_file(wrong_path).unwrap();
    fs::write(&event_path, &original_event_bytes).unwrap();

    let mut fork: codeguard_core::TaskLifecycleRecord =
        serde_json::from_slice(&fs::read(&event_path).unwrap()).unwrap();
    let mut fork_evidence: Value = serde_json::from_slice(
        &fs::read(p.root.join(r["evidence_ref"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    fork_evidence["current_source_sha256"] = json!("b".repeat(64));
    let fork_bytes = serde_json::to_vec_pretty(&fork_evidence).unwrap();
    let fork_sha = sha(&fork_bytes);
    fs::write(
        p.root.join(format!(
            ".codeguard/state/resolution_evidence/{fork_sha}.json"
        )),
        &fork_bytes,
    )
    .unwrap();
    fork.evidence_sha256 = Some(fork_sha.clone());
    fork.event.kind = codeguard_core::TaskLifecycleKind::Resolved {
        cause: codeguard_core::ResolutionCause::CodeFixed,
        evidence_sha256: fork_sha,
    };
    fork.event.event_id.clear();
    fork.event.event_id = format!("event-{}", sha(&serde_json::to_vec(&fork).unwrap()));
    let fork_path = event_path
        .parent()
        .unwrap()
        .join(format!("lifecycle-{}.json", fork.event.event_id));
    fs::write(&fork_path, serde_json::to_vec_pretty(&fork).unwrap()).unwrap();
    assert!(matches!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_reconciliation_required")
    ));
    fs::remove_file(fork_path).unwrap();
    let evidence = p.root.join(r["evidence_ref"].as_str().unwrap());
    fs::remove_file(evidence).unwrap();
    assert!(matches!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_evidence_missing")
    ));
}
#[test]
#[ignore = "需要本机已安装 Zig 0.16.0；受保护宿主密钥来源仍使用测试夹具"]
fn real_zig_original_and_fixed_source_resolution() {
    let p = Project::new();
    let tool = Path::new("/opt/homebrew/bin/zig").canonicalize().unwrap();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    fs::write(p.root.join("app.zig"), BAD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
}

#[test]
fn host_resolution_consumes_the_finished_attempt_without_releasing_its_lease() {
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
    command(&["task", "verify", &p.id, "--zig-tool", tool_arg]);
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
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
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
fn untrusted_or_expired_host_context_and_duplicate_policy_are_rejected_before_execution() {
    let p = Project::new();
    let marker = p.root.join("executed");
    let tool = p.tool(&format!("/usr/bin/touch '{}'\nexit 0", marker.display()));
    let raw = serde_json::to_vec(&p.policy(&tool)).unwrap();
    for host_case in [
        "wrong_key",
        "revoked",
        "clock_missing",
        "expired",
        "rollback",
    ] {
        assert!(
            p.verify_raw(&tool, &raw, None, host_case).is_err(),
            "{host_case}"
        );
    }
    let duplicated = format!(
        "{{\"schema_version\":\"1.0.0\",{}",
        std::str::from_utf8(&raw)
            .unwrap()
            .strip_prefix('{')
            .unwrap()
    );
    assert!(
        p.verify_raw(&tool, duplicated.as_bytes(), None, "valid")
            .is_err()
    );
    assert!(!marker.exists());
    assert!(p.events().is_empty());
}
fn export_artifacts(p: &Project, receipt: &Value, policy: &Value, name: &str) {
    if let Some(dir) = std::env::var_os("CODEGUARD_RESOLUTION_ARTIFACT_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{name}-receipt.json")),
            serde_json::to_vec_pretty(receipt).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join(format!("{name}-policy.json")),
            serde_json::to_vec_pretty(policy).unwrap(),
        )
        .unwrap();
        fs::copy(
            p.root.join(receipt["evidence_ref"].as_str().unwrap()),
            dir.join(format!("{name}-evidence.json")),
        )
        .unwrap();
        fs::write(
            dir.join(format!("{name}-events.json")),
            serde_json::to_vec_pretty(&p.events()).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn source_changed_during_original_replay_invalidates_closure_and_preserves_failure() {
    let p = Project::new();
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let tool=p.tool(&format!("input=$(/bin/cat)\nprintf 'pub fn main( void {{\\n' > '{}'\ncase \"$input\" in *'main( void'*) printf '<stdin>:1:13: error: expected token\\n' >&2; exit 1;; *) exit 0;; esac",p.root.join("app.zig").display()));
    let r = p.verify(&tool, &p.policy(&tool)).unwrap();
    assert_eq!(r["state"], "verification_required");
    assert_eq!(r["outcome"], "inputs_stale");
    assert!(
        !p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "resolved")
    );
}

#[test]
fn invalid_original_native_coordinates_never_prove_code_fix() {
    let p = Project::new();
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let tool=p.tool("input=$(/bin/cat)\ncase \"$input\" in *'main( void'*) printf '<stdin>:999:1: error: invalid position\\n' >&2; exit 1;; *) exit 0;; esac");
    let r = p.verify(&tool, &p.policy(&tool)).unwrap();
    assert_eq!(r["state"], "verification_required");
    assert_eq!(r["outcome"], "native_incomplete");
}
