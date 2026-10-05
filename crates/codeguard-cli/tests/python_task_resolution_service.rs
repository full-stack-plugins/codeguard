#![cfg(all(unix, feature = "wasm-precheck"))]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, PythonTaskResolutionRequest,
    verify_python_task_resolution,
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
const BAD: &[u8] = b"def run():\n";
const GOOD: &[u8] = b"def run():\n    return 1\n";
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Project {
    root: PathBuf,
    id: String,
    workspace: String,
    original: Value,
    original_source: Vec<u8>,
    target_version: String,
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Project {
    fn new(generic: bool) -> Self {
        Self::with_source(generic, BAD, "py312")
    }
    fn with_source(generic: bool, source: &[u8], target: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-python-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), source).unwrap();
        fs::write(
            root.join("ruff.toml"),
            format!("target-version = '{target}'\n[lint]\nselect = ['F401']\n"),
        )
        .unwrap();
        let cli = env!("CARGO_BIN_EXE_codeguard");
        assert_eq!(
            Command::new(cli)
                .args(["init"])
                .arg(&root)
                .args(["--apply", "--format=json"])
                .output()
                .unwrap()
                .status
                .code(),
            Some(3)
        );
        let mut cmd = Command::new(cli);
        if generic {
            cmd.args(["check", "python"]);
        } else {
            cmd.args(["lint", "python"]);
        }
        assert_eq!(
            cmd.arg(&root)
                .arg("--format=json")
                .env("PATH", "")
                .output()
                .unwrap()
                .status
                .code(),
            Some(3)
        );
        let fact: Value = fs::read_dir(root.join(".codeguard/findings"))
            .unwrap()
            .filter_map(|e| {
                serde_json::from_slice::<Value>(
                    &fs::read(e.ok()?.path().join("finding.json")).ok()?,
                )
                .ok()
            })
            .find(|f| {
                f["reason_code"] == "python_syntax_confirmation_needed" && f["scope"] == "app.py"
            })
            .unwrap();
        let original = serde_json::from_slice(
            &fs::read(root.join(format!(
                ".codeguard/reports/{}.json",
                fact["first_run_id"].as_str().unwrap()
            )))
            .unwrap(),
        )
        .unwrap();
        Self {
            root,
            id: fact["id"].as_str().unwrap().into(),
            workspace: fact["workspace_id"].as_str().unwrap().into(),
            original,
            original_source: source.to_vec(),
            target_version: target.into(),
        }
    }
    fn policy(&self, tool: &Path) -> Value {
        let source = if self.original["report_type"] == "python_syntax_confirmation_observation" {
            &self.original["source_sha256"]
        } else {
            &self.original["observations"][0]["source_sha256"]
        };
        let grammar = if self.original["report_type"] == "python_syntax_confirmation_observation" {
            &self.original["grammar_sha256"]
        } else {
            &self.original["observations"][0]["grammar_sha256"]
        };
        json!({"schema_version":"1.5.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"python.ruff","scope":"app.py"},"policy_revision":"p1","original_report_sha256":sha(&serde_json::to_vec_pretty(&self.original).unwrap()),"original_source_sha256":source,"grammar_sha256":grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"invalid-syntax","native_version":"ruff 0.16.8","target_version":self.target_version,"configuration_ref":"ruff.toml","configuration_sha256":sha(&fs::read(self.root.join("ruff.toml")).unwrap())})
    }
    fn verify(&self, tool: &Path, policy: &Value, revoked: bool) -> Result<Value, &'static str> {
        let raw = serde_json::to_vec(policy).unwrap();
        let pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        let payload=json!({"schema_version":"1.0","workspace_id":self.workspace,"policy_revision":"p1","baseline_commit":"a".repeat(40),"revision_sequence":1,"issued_at":100,"expires_at":300,"snapshot_sha256":sha(&raw)}).to_string();
        let mut message = b"codeguard.approval.v1\0python-fixture\0".to_vec();
        message.extend_from_slice(payload.as_bytes());
        let signature = pair
            .sign(&message)
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let envelope=json!({"schema_version":"1.0","key_id":"python-fixture","approval_json":payload,"signature_hex":signature}).to_string();
        let trust = ApprovalTrustKey {
            key_id: "python-fixture".into(),
            public_key: pair.public_key().as_ref().try_into().unwrap(),
            valid_from: 1,
            valid_until: 400,
            revoked,
        };
        let context = ApprovalVerificationContext {
            workspace_id: &self.workspace,
            policy_revision: "p1",
            baseline_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            now_unix: Some(150),
            minimum_sequence: 1,
            max_lifetime_seconds: 200,
        };
        verify_python_task_resolution(&PythonTaskResolutionRequest {
            root: &self.root,
            task_id: &self.id,
            tool,
            original_source: &self.original_source,
            policy_bytes: &raw,
            envelope_bytes: envelope.as_bytes(),
            trust: &trust,
            context: &context,
            deadline: Instant::now() + Duration::from_secs(45),
            borrowed_lease: None,
        })
    }
}
#[test]
fn invalid_python_policy_and_revoked_host_do_not_execute_native_tool() {
    let p = Project::new(false);
    let tool = p.root.join("ruff-marker");
    fs::write(&tool, "#!/bin/sh\nprintf executed > \"$0.executed\"\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let policy = p.policy(&tool);
    assert_eq!(p.verify(&tool, &policy, true), Err("approval_key_revoked"));
    for (key, value) in [
        ("schema_version", "1.0.0"),
        ("target_version", "py316"),
        ("native_rule_id", "F401"),
        ("native_version", "ruff 0.1.0"),
        ("configuration_sha256", "bad"),
    ] {
        let mut bad = policy.clone();
        bad[key] = json!(value);
        assert!(p.verify(&tool, &bad, false).is_err(), "{key}");
    }
    assert!(!tool.with_extension("executed").exists());
}
#[test]
#[ignore = "requires installed Ruff 0.16.8 via CODEGUARD_RUFF_BIN; host trust uses independent fixtures"]
fn actual_python_resolution_closes_idempotently_and_reopens_both_original_families() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").unwrap());
    for generic in [false, true] {
        let p = Project::new(generic);
        let policy = p.policy(&tool);
        let still = p.verify(&tool, &policy, false).unwrap();
        assert_eq!(still["outcome"], "still_present");
        let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
        assert_eq!(brief["task_id"], p.id, "{brief}");
        assert_eq!(brief["action_id"], "repair-source", "{brief}");
        fs::write(p.root.join("app.py"), GOOD).unwrap();
        let fixed = p.verify(&tool, &policy, false).unwrap();
        assert_eq!(fixed["outcome"], "code_fixed");
        assert_eq!(fixed["state"], "resolved");
        assert_eq!(fixed["delivery_decision"], "not_evaluated");
        if let Some(dir) = std::env::var_os("CODEGUARD_RESOLUTION_ARTIFACT_DIR") {
            let dir = PathBuf::from(dir);
            fs::create_dir_all(&dir).unwrap();
            let name = if generic {
                "python-generic"
            } else {
                "python-dedicated"
            };
            fs::write(
                dir.join(format!("{name}-policy.json")),
                serde_json::to_vec_pretty(&policy).unwrap(),
            )
            .unwrap();
            fs::copy(
                p.root.join(fixed["evidence_ref"].as_str().unwrap()),
                dir.join(format!("{name}-evidence.json")),
            )
            .unwrap();
        }

        let again = p.verify(&tool, &policy, false).unwrap();
        assert_eq!(again["event_ref"], fixed["event_ref"]);
        fs::write(p.root.join("app.py"), BAD).unwrap();
        let ordinary = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify", &p.id])
            .arg(&p.root)
            .arg("--format=json")
            .arg("--ruff-tool")
            .arg(&tool)
            .output()
            .unwrap();
        assert_eq!(ordinary.status.code(), Some(3));
        let reopened_count =
            fs::read_dir(p.root.join(format!(".codeguard/findings/{}/events", p.id)))
                .unwrap()
                .filter_map(|e| {
                    serde_json::from_slice::<Value>(&fs::read(e.ok()?.path()).ok()?).ok()
                })
                .filter(|e| {
                    e["event"]["kind"] == "reopened" || e["event"]["kind"]["event"] == "reopened"
                })
                .count();
        assert_eq!(reopened_count, 1, "普通task verify必须重开同一任务");
        let reopened = p.verify(&tool, &policy, false).unwrap();
        assert_eq!(reopened["outcome"], "still_present");
        assert_eq!(reopened["state"], "open");
    }
}

#[test]
#[ignore = "requires installed Ruff0.16.8 via CODEGUARD_RUFF_BIN; Python3.14 target uses native syntax oracle"]
fn actual_legal_template_string_requires_false_positive_review_without_source_edits() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").unwrap());
    let source = b"message = t\"hello\"\n";
    for generic in [false, true] {
        let p = Project::with_source(generic, source, "py314");
        let policy = p.policy(&tool);
        let receipt = p.verify(&tool, &policy, false).unwrap();
        assert_eq!(receipt["outcome"], "false_positive_review_required");
        assert_eq!(receipt["delivery_decision"], "not_evaluated");
        assert_ne!(receipt["state"], "resolved");
        assert_eq!(fs::read(p.root.join("app.py")).unwrap(), source);
        let evidence: Value = serde_json::from_slice(
            &fs::read(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        assert_eq!(evidence["original_native"]["status"], "completed");
        assert_eq!(evidence["current_native"]["status"], "completed");
        assert_eq!(
            evidence["original_source_sha256"],
            evidence["current_source_sha256"]
        );
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("next")
            .arg(&p.root)
            .arg("--format=json")
            .output()
            .unwrap();
        let next: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(next["repair_brief"]["task_id"], p.id, "{next}");
        assert_eq!(next["repair_brief"]["disposition"], "verification_required");
        assert!(
            next["repair_brief"]["step"]
                .as_str()
                .unwrap()
                .contains("WASM"),
            "{next}"
        );

        // 历史反证不能掩盖之后的当前源码变化。
        fs::write(p.root.join("app.py"), BAD).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("next")
            .arg(&p.root)
            .arg("--format=json")
            .output()
            .unwrap();
        let changed: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(changed["repair_brief"]["task_id"], p.id, "{changed}");
        assert_eq!(
            changed["repair_brief"]["verification_invalidated_reason"],
            "source_input_changed_or_unavailable"
        );
        assert!(
            changed["repair_brief"]["step"]
                .as_str()
                .unwrap()
                .contains("源码或配置已变化"),
            "{changed}"
        );
        fs::write(p.root.join("app.py"), source).unwrap();
        fs::write(
            p.root.join("ruff.toml"),
            "target-version = 'py313'\n[lint]\nselect = ['F401']\n",
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("next")
            .arg(&p.root)
            .arg("--format=json")
            .output()
            .unwrap();
        let changed: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            changed["repair_brief"]["verification_invalidated_reason"],
            "configuration_input_changed_or_unavailable"
        );
        assert!(
            changed["repair_brief"]["step"]
                .as_str()
                .unwrap()
                .contains("源码或配置已变化"),
            "{changed}"
        );
    }
}
