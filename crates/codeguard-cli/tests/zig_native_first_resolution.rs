#![cfg(unix)]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, ZigTaskResolutionRequest,
    verify_zig_task_resolution,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const BAD: &[u8] = b"pub fn main( void {\n";
const GOOD: &[u8] = b"const Empty = struct {};\n";
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
const NORMAL_TOOL: &str = "if [ -f \"$0.clean\" ]; then /bin/cat >/dev/null; exit 0; fi\nif [ -f \"$0.invalid\" ]; then /bin/cat >/dev/null; printf unknown >&2; exit 1; fi\ninput=$(/bin/cat)\ncase \"$input\" in *'pub fn main'*) printf '<stdin>:1:13: error: expected type\\npub fn main( void {\\n            ^\\n' >&2; exit 1;; *) exit 0;; esac";
fn write_tool(root: &Path, body: &str) -> PathBuf {
    let p = root.join("zig");
    fs::write(
        &p,
        format!("#!/bin/sh\nprintf x >> \"$0.calls\"\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\n{body}\n")
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
            "cg-zig-resolution-{}-{}",
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
        assert!(native_first);
        let tool = native_tool
            .map(Path::to_path_buf)
            .unwrap_or_else(|| write_tool(&root, NORMAL_TOOL));
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all"])
            .arg(&root)
            .arg("--zig-tool")
            .arg(&tool)
            .args(["--format=json", "--timeout", "60s"])
            .env("PATH", &root)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&o.stdout).unwrap();
        let scan = &r["native_results"]["zig_lint"];
        assert_eq!(
            scan["files"][0]["native"]["status"], "diagnostics_observed",
            "{r}"
        );
        let id = scan["files"][0]["task_id"].as_str().unwrap().to_owned();
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
        json!({"schema_version":"1.4.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"syntax.native_confirmation","scope":"app.zig"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":self.grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"zig.ast_check.error","native_version":"0.16.0"})
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
fn zig_native_resolution_closes_replays_and_public_recheck_reopens() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    assert!(policy["grammar_sha256"].is_null());
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    assert_eq!(r["outcome"], "code_fixed");
    assert_eq!(r["delivery_decision"], "not_evaluated");
    assert_eq!(p.events().len(), 2);
    assert_eq!(
        p.verify(&tool, &policy).unwrap()["event_ref"],
        r["event_ref"]
    );
    assert_eq!(p.events().len(), 2);
    fs::write(p.root.join("app.zig"), BAD).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--zig-tool")
        .arg(&tool)
        .args(["--format=json", "--timeout", "60s"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
    assert_eq!(p.events().len(), 3);
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "resolved");
    assert_eq!(p.events().len(), 4);
}

#[test]
fn zig_policy_identity_and_host_trust_reject_before_native_execution() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    let calls = fs::read(tool.with_extension("calls")).unwrap();
    for (field, value) in [
        ("schema_version", "1.0.0"),
        ("native_rule_id", "erlang.syntax.error"),
        ("native_version", "OTP 28"),
        ("grammar_sha256", &"a".repeat(64)),
    ] {
        let mut wrong = policy.clone();
        wrong[field] = json!(value);
        assert!(p.verify(&tool, &wrong).is_err(), "{field}");
        assert!(p.events().is_empty());
        assert_eq!(fs::read(tool.with_extension("calls")).unwrap(), calls);
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
        assert!(p.events().is_empty());
        assert_eq!(fs::read(tool.with_extension("calls")).unwrap(), calls);
    }
}

#[test]
#[ignore = "需要本机已安装 Zig 0.16.0，宿主签名是测试夹具"]
fn real_zig016_resolution_and_recurrence() {
    let tool = PathBuf::from(std::env::var("CODEGUARD_ZIG_BIN").unwrap());
    let p = Project::create_with_tool(true, Some(&tool));
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    assert_eq!(r["outcome"], "code_fixed");
    if let Ok(dir) = std::env::var("CODEGUARD_ZIG_RESOLUTION_REPORT_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        for (name, value) in [
            ("policy", policy.clone()),
            ("receipt", r.clone()),
            (
                "evidence",
                serde_json::from_slice(
                    &fs::read(p.root.join(r["evidence_ref"].as_str().unwrap())).unwrap(),
                )
                .unwrap(),
            ),
        ] {
            fs::write(
                dir.join(format!("{name}.json")),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
        }
    }
    fs::write(p.root.join("app.zig"), BAD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "open");
    assert_eq!(r["outcome"], "still_present");
}

#[test]
fn zig_original_counterexample_and_incomplete_native_observation_do_not_close() {
    for (mode, expected) in [
        ("clean", "false_positive_review_required"),
        ("invalid", "native_incomplete"),
    ] {
        let p = Project::new();
        let tool = p.normal_tool();
        let policy = p.policy(&tool);
        fs::write(tool.with_extension(mode), b"fixture mode").unwrap();
        fs::write(p.root.join("app.zig"), GOOD).unwrap();
        let r = p.verify(&tool, &policy).unwrap();
        assert_eq!(r["outcome"], expected, "{r}");
        assert_ne!(r["state"], "resolved");
        assert!(
            !p.events()
                .iter()
                .any(|e| e["event"]["kind"]["event"] == "resolved")
        );
        if mode == "clean" {
            let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
            let step = brief["step"].as_str().unwrap();
            assert!(step.contains("首次原生"), "{brief}");
            assert!(!step.contains("grammar"), "{brief}");
            let calls = fs::read(tool.with_extension("calls")).unwrap();
            let records = p.events();
            let lease_path = p
                .root
                .join(format!(".codeguard/state/leases/{}.json", p.id));
            let lease = fs::read(&lease_path).unwrap();
            for operation in ["next", "task-show-json", "task-show-human"] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
                if operation == "next" {
                    command.arg("next").arg(&p.root).arg("--format=json");
                } else {
                    command.args(["task", "show", &p.id]).arg(&p.root).arg(
                        if operation == "task-show-json" {
                            "--format=json"
                        } else {
                            "--format=human"
                        },
                    );
                }
                let o = command.output().unwrap();
                assert!(matches!(o.status.code(), Some(0 | 3)), "{o:?}");
                if operation == "task-show-human" {
                    let text = String::from_utf8(o.stdout).unwrap();
                    assert!(text.contains(step), "{text}");
                    assert!(!text.contains("grammar 误报"), "{text}");
                } else {
                    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
                    let projected = if operation == "next" {
                        &r["repair_brief"]
                    } else {
                        &r["task"]
                    };
                    assert_eq!(projected["step"], step, "{r}");
                    assert_eq!(projected["disposition"], "verification_required");
                }
            }
            assert_eq!(fs::read(tool.with_extension("calls")).unwrap(), calls);
            assert_eq!(p.events(), records);
            assert_eq!(fs::read(lease_path).unwrap(), lease);
        }
    }
}

#[test]
fn zig_changed_native_tool_cannot_close_the_original_task() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(&tool, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::write(p.root.join("app.zig"), GOOD).unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_resolution_tool_mismatch")
    );
    assert!(p.events().is_empty());
    assert_eq!(
        p.verify(&tool, &p.policy(&tool)),
        Err("task_resolution_original_identity_mismatch")
    );
}
