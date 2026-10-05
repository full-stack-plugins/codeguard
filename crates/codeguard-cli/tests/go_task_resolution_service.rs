#![cfg(all(unix, feature = "wasm-precheck"))]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, GoTaskResolutionRequest,
    verify_go_task_resolution,
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
const BAD: &[u8] = b"func f() {}\n";
const GOOD: &[u8] = b"package main\nfunc f() {}\n";
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
            "cg-go-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.go"), BAD).unwrap();
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
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.go"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
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
        let tool = self.root.join("go");
        fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = version ]; then if [ \"$#\" = 1 ]; then printf 'go version go1.23.4 test/host\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi; exit 0; fi\nexit 2\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let fmt = self.root.join("gofmt");
        fs::write(&fmt, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&fmt, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn normal_tool(&self) -> PathBuf {
        self.tool("input=$(/bin/cat)\ncase \"$input\" in *'package main'*) printf '%s\\n' \"$input\"; exit 0;; *) printf '/dev/stdin:1:1: expected package\\n' >&2; exit 2;; esac")
    }
    fn policy(&self, tool: &Path) -> Value {
        let fmt = tool.parent().unwrap().join("gofmt").canonicalize().unwrap();
        let fmt_sha = sha(&fs::read(&fmt).unwrap());
        let mut hash = Sha256::new();
        hash.update(fmt.as_os_str().as_encoded_bytes());
        hash.update([0]);
        hash.update(fmt_sha.as_bytes());
        json!({"schema_version":"1.6.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"syntax.native_confirmation","scope":"app.go"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":self.grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"go.syntax","native_version":"go1.23.4","gofmt_sha256":fmt_sha,"companion_binding_sha256":format!("{:x}",hash.finalize())})
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
        let request = GoTaskResolutionRequest {
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
        };
        if host_case == "old_zig" {
            codeguard_cli::verify_zig_task_resolution(&request)
        } else {
            verify_go_task_resolution(&request)
        }
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
fn signed_go_pair_closes_idempotently_and_native_recurrence_reopens() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["state"], "resolved", "{receipt}");
    assert_eq!(receipt["delivery_decision"], "not_evaluated");
    let repeated = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["event_ref"], repeated["event_ref"]);
    assert_eq!(p.events().len(), 2);
    fs::write(p.root.join("app.go"), BAD).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--go-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let local: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(local["observation"], "still_blocked");
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "resolved");
}
#[test]
fn signed_companion_mismatch_is_rejected_before_processes() {
    let p = Project::new();
    let marker = p.root.join("executed");
    let tool = p.tool(&format!("/usr/bin/touch '{}'\nexit 0", marker.display()));
    for field in ["gofmt_sha256", "companion_binding_sha256"] {
        let mut policy = p.policy(&tool);
        policy[field] = json!("0".repeat(64));
        assert_eq!(
            p.verify(&tool, &policy),
            Err("task_resolution_companion_mismatch")
        );
    }
    let mut policy = p.policy(&tool);
    policy.as_object_mut().unwrap().remove("gofmt_sha256");
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_resolution_policy_invalid")
    );
    assert!(!marker.exists());
    assert!(p.events().is_empty());
}
#[test]
fn original_zero_diagnostics_requires_review_and_missing_evidence_is_rejected() {
    let p = Project::new();
    let tool = p.tool("/bin/cat\nexit 0");
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    let policy = p.policy(&tool);
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["outcome"], "false_positive_review_required");
    assert_eq!(receipt["state"], "verification_required");
    assert_eq!(
        p.verify(&tool, &policy).unwrap()["outcome"],
        "false_positive_review_required"
    );
    fs::remove_file(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_evidence_missing")
    );
}
#[test]
#[ignore = "需要已安装Go1.23.4；宿主密钥为测试信任夹具"]
fn real_go_pair_fixed_source_resolution_and_recurrence() {
    let p = Project::new();
    let tool = Path::new("/usr/local/go/bin/go").canonicalize().unwrap();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["state"], "resolved");
    export(&p, &policy, &receipt, "resolved");
    fs::write(p.root.join("app.go"), BAD).unwrap();
    let reopened = p.verify(&tool, &policy).unwrap();
    assert_eq!(reopened["state"], "open");
    export(&p, &policy, &reopened, "reopened");
}

#[test]
fn companion_mutation_during_original_probe_preserves_stale_evidence() {
    let p = Project::new();
    let tool = p.tool("/bin/cat >/dev/null\nprintf '\\n# mutated\\n' >> \"$0\"\nexit 2");
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["outcome"], "inputs_stale");
    assert_eq!(receipt["state"], "verification_required");
    assert!(
        !p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "resolved")
    );
}
#[test]
fn wrong_host_and_cross_version_policies_do_not_run_tools() {
    let p = Project::new();
    let marker = p.root.join("executed");
    let tool = p.tool(&format!("/usr/bin/touch '{}'\nexit 0", marker.display()));
    let policy = p.policy(&tool);
    for case in [
        "wrong_key",
        "revoked",
        "clock_missing",
        "expired",
        "rollback",
    ] {
        assert!(
            p.verify_raw(&tool, &serde_json::to_vec(&policy).unwrap(), None, case)
                .is_err(),
            "{case}"
        );
    }
    let mut old = policy.clone();
    old["schema_version"] = json!("1.3.0");
    assert_eq!(
        p.verify(&tool, &old),
        Err("task_resolution_policy_scope_mismatch")
    );
    assert!(!marker.exists());
    assert!(p.events().is_empty());
}

fn export(p: &Project, policy: &Value, receipt: &Value, suffix: &str) {
    let Some(dir) = std::env::var_os("CODEGUARD_GO_RESOLUTION_CAPTURE") else {
        return;
    };
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir).unwrap();
    for (kind, bytes) in [
        ("policy", serde_json::to_vec_pretty(policy).unwrap()),
        ("receipt", serde_json::to_vec_pretty(receipt).unwrap()),
        (
            "evidence",
            fs::read(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
        ),
    ] {
        fs::write(
            dir.join(format!("go1234-resolution-2026-10-05-{suffix}-{kind}.json")),
            bytes,
        )
        .unwrap();
    }
}
#[test]
fn legacy_zig_policy_rejects_go_companion_extensions() {
    let p = Project::new();
    let tool = p.normal_tool();
    let mut policy = p.policy(&tool);
    policy["schema_version"] = json!("1.0.0");
    policy["native_version"] = json!("0.16.0");
    policy["native_rule_id"] = json!("zig.ast_check.error");
    assert_eq!(
        p.verify_raw(
            &tool,
            &serde_json::to_vec(&policy).unwrap(),
            None,
            "old_zig"
        ),
        Err("task_resolution_policy_invalid")
    );
    assert!(p.events().is_empty());
}

#[test]
fn rewritten_closed_evidence_cannot_replace_the_approved_companion() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.go"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    let path = p.root.join(receipt["evidence_ref"].as_str().unwrap());
    let mut evidence: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    evidence["current_native"]["gofmt_sha256"] = json!("0".repeat(64));
    let bytes = serde_json::to_vec_pretty(&evidence).unwrap();
    let changed = sha(&bytes);
    fs::write(
        path.parent().unwrap().join(format!("{changed}.json")),
        bytes,
    )
    .unwrap();
    let event_path = p.root.join(receipt["event_ref"].as_str().unwrap());
    let mut record: codeguard_core::TaskLifecycleRecord =
        serde_json::from_slice(&fs::read(&event_path).unwrap()).unwrap();
    record.evidence_sha256 = Some(changed.clone());
    record.event.kind = codeguard_core::TaskLifecycleKind::Resolved {
        cause: codeguard_core::ResolutionCause::CodeFixed,
        evidence_sha256: changed,
    };
    record.event.event_id.clear();
    record.event.event_id = format!("event-{}", sha(&serde_json::to_vec(&record).unwrap()));
    fs::write(
        event_path
            .parent()
            .unwrap()
            .join(format!("lifecycle-{}.json", record.event.event_id)),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    fs::remove_file(event_path).unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_lifecycle_evidence_binding_invalid")
    );
}
