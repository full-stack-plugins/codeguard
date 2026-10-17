#![cfg(unix)]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, RustTaskResolutionRequest,
    verify_rust_task_resolution,
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
const BAD: &[u8] = b"pub fn f( {\n";
const GOOD: &[u8] = b"pub fn f() {}\n";
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
struct Project {
    root: PathBuf,
    id: String,
    workspace: String,
    original_report: String,
    edition_context: Value,
    tool: PathBuf,
    grammar: Option<String>,
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Project {
    fn new() -> Self {
        Self::new_with_origin(false)
    }
    fn new_with_origin(wasm_first: bool) -> Self {
        let host_size = fs::metadata(std::env::current_exe().unwrap())
            .unwrap()
            .len();
        assert!(
            host_size <= 256 * 1024 * 1024,
            "SDK test host exceeds the product artifact budget: {host_size} bytes; use CARGO_PROFILE_TEST_DEBUG=0 without widening the product limit"
        );
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-rust-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.rs"), BAD).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        let tool = if let Ok(tool) = std::env::var("CODEGUARD_RUST_RESOLUTION_TOOL") {
            PathBuf::from(tool)
        } else {
            let tool = root.join("rustfmt");
            fs::write(&tool, format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\nmode=$(/bin/cat '{}/probe-mode' 2>/dev/null)\nif [ \"$mode\" = mutate ]; then printf '[package]\\nname=\"sample\"\\nversion=\"0.1.0\"\\nedition=\"2024\"\\n' > '{}/Cargo.toml'; fi\ninput=$(/bin/cat)\n[ \"$mode\" = zero ] && {{ printf 'formatted\\n'; exit 0; }}\ncase \"$input\" in *'pub fn f( {{'*) printf 'error: expected token\\n --> <stdin>:1:1\\n' >&2; exit 1;; esac\nprintf 'formatted\\n'\n", root.display(),root.display())).unwrap();
            fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
            tool
        };
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        if !wasm_first {
            command
                .arg("hook")
                .arg("execute")
                .arg(&root)
                .arg("--rustfmt-tool")
                .arg(&tool);
        } else {
            command.arg("hook").arg("execute").arg(&root);
        }
        let mut child = command
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", "")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.rs"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
        let o = child.wait_with_output().unwrap();
        let r: Value = serde_json::from_slice(&o.stdout).unwrap();
        let task_id = if wasm_first {
            &r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        } else {
            &r["local_feedback"]["rust_syntax"]["files"][0]["task_id"]
        };
        assert!(task_id.is_string(), "{r}");
        let id = task_id.as_str().unwrap().to_owned();
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
        let edition_context =
            codeguard_cli::rust_project_edition::RustProjectEdition::capture(&root, "app.rs")
                .unwrap()
                .report();
        Self {
            root,
            id,
            workspace: fact["workspace_id"].as_str().unwrap().into(),
            original_report: fact["first_report_sha256"].as_str().unwrap().into(),
            edition_context,
            grammar: original["observations"][0]["grammar_sha256"]
                .as_str()
                .map(str::to_owned),
            tool,
        }
    }
    fn normal_tool(&self) -> PathBuf {
        self.tool.clone()
    }
    fn policy(&self, tool: &Path) -> Value {
        json!({"schema_version":if self.grammar.is_some(){"1.8.0"}else{"1.7.0"},"report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"syntax.native_confirmation","scope":"app.rs"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":self.grammar,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"rust.syntax","native_version":"rustfmt 1.9.0-stable","edition_context":self.edition_context})
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
        let request = RustTaskResolutionRequest {
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
        let result = if host_case == "old_zig" {
            codeguard_cli::verify_zig_task_resolution(&request)
        } else {
            verify_rust_task_resolution(&request)
        };
        if let (Ok(dir), Ok(receipt)) =
            (std::env::var("CODEGUARD_RUST_RESOLUTION_CAPTURE"), &result)
        {
            let evidence: Value = serde_json::from_slice(
                &fs::read(self.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
            )
            .unwrap();
            fs::write(
                PathBuf::from(dir).join(format!(
                    "resolution-{}.json",
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
                serde_json::to_vec_pretty(
                    &json!({"policy":policy_value(raw),"receipt":receipt,"evidence":evidence}),
                )
                .unwrap(),
            )
            .unwrap();
        }
        result
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
fn signed_rust_pair_closes_idempotently_and_native_recurrence_reopens() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    let receipt = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["state"], "resolved", "{receipt}");
    assert_eq!(receipt["delivery_decision"], "not_evaluated");
    let repeated = p.verify(&tool, &policy).unwrap();
    assert_eq!(receipt["event_ref"], repeated["event_ref"]);
    assert_eq!(p.events().len(), 2);
    fs::write(p.root.join("app.rs"), BAD).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--rustfmt-tool")
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
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "resolved");
}
#[test]
fn changed_edition_and_forged_context_cannot_close() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    let mut forged = policy.clone();
    forged["edition_context"]["edition"] = json!("2024");
    assert!(p.verify(&tool, &forged).is_err());
    assert!(p.events().is_empty());
    fs::write(
        p.root.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_resolution_edition_context_mismatch")
    );
    assert!(p.events().is_empty());
}

#[test]
fn trust_failures_are_rejected_without_execution_or_events() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    for case in [
        "wrong_key",
        "revoked",
        "clock_missing",
        "expired",
        "rollback",
        "old_zig",
    ] {
        assert!(
            p.verify_raw(&tool, &serde_json::to_vec(&policy).unwrap(), None, case)
                .is_err(),
            "{case}"
        );
        assert!(p.events().is_empty());
    }
    let mut wrong = policy.clone();
    wrong["tool_sha256"] = json!("0".repeat(64));
    assert_eq!(
        p.verify(&tool, &wrong),
        Err("task_resolution_original_identity_mismatch")
    );
    let mut extra = policy.clone();
    extra["gofmt_sha256"] = json!("0".repeat(64));
    assert_eq!(
        p.verify(&tool, &extra),
        Err("task_resolution_policy_invalid")
    );
}
#[test]
fn native_counterexample_and_mid_check_context_mutation_never_resolve() {
    assert!(
        std::env::var("CODEGUARD_RUST_RESOLUTION_TOOL").is_err(),
        "controlled mutation fixture only"
    );
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    fs::write(p.root.join("probe-mode"), "zero").unwrap();
    let review = p.verify(&tool, &policy).unwrap();
    assert_eq!(review["outcome"], "false_positive_review_required");
    assert_eq!(review["state"], "verification_required");
    assert_eq!(
        p.verify(&tool, &policy).unwrap()["outcome"],
        "false_positive_review_required"
    );
    fs::write(p.root.join("probe-mode"), "mutate").unwrap();
    let stale = p.verify(&tool, &policy).unwrap();
    assert_eq!(stale["outcome"], "inputs_stale");
    assert_ne!(stale["state"], "resolved");
}
#[test]
#[ignore = "需要独立固定的真实Rustfmt1.9.0-stable；签名密钥仍为宿主测试夹具"]
fn real_rustfmt_resolution_and_recurrence() {
    assert!(std::env::var("CODEGUARD_RUST_RESOLUTION_TOOL").is_ok());
    signed_rust_pair_closes_idempotently_and_native_recurrence_reopens();
}

fn policy_value(raw: &[u8]) -> Value {
    serde_json::from_slice(raw).unwrap()
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn wasm_first_rust_task_confirms_resolves_and_reopens_with_original_grammar() {
    let p = Project::new_with_origin(true);
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    assert!(p.grammar.is_some());
    assert_eq!(policy["schema_version"], "1.8.0");
    let still = p.verify(&tool, &policy).unwrap();
    assert_eq!(still["state"], "open");
    assert_eq!(still["outcome"], "still_present");
    fs::write(p.root.join("app.rs"), GOOD).unwrap();
    let fixed = p.verify(&tool, &policy).unwrap();
    assert_eq!(fixed["state"], "resolved");
    assert_eq!(
        fixed["event_ref"],
        p.verify(&tool, &policy).unwrap()["event_ref"]
    );
    fs::write(p.root.join("app.rs"), BAD).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--rustfmt-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    assert_eq!(p.verify(&tool, &policy).unwrap()["state"], "open");
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn wasm_first_grammar_origin_and_edition_are_bound_before_native_execution() {
    let p = Project::new_with_origin(true);
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    for grammar in [Value::Null, json!("0".repeat(64))] {
        let mut wrong = policy.clone();
        wrong["grammar_sha256"] = grammar;
        assert_eq!(
            p.verify(&tool, &wrong),
            Err("task_resolution_original_identity_mismatch")
        );
    }
    let mut missing = policy.clone();
    missing.as_object_mut().unwrap().remove("grammar_sha256");
    assert_eq!(
        p.verify(&tool, &missing),
        Err("task_resolution_policy_invalid")
    );
    let mut origin = policy.clone();
    origin["schema_version"] = json!("1.7.0");
    origin["grammar_sha256"] = Value::Null;
    assert_eq!(
        p.verify(&tool, &origin),
        Err("task_resolution_original_identity_mismatch")
    );
    let mut tool_changed = policy.clone();
    tool_changed["tool_sha256"] = json!("0".repeat(64));
    assert_eq!(
        p.verify(&tool, &tool_changed),
        Err("task_resolution_tool_mismatch")
    );
    assert!(p.events().is_empty());
    fs::write(
        p.root.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    assert_eq!(
        p.verify(&tool, &policy),
        Err("task_resolution_edition_context_mismatch")
    );
    assert!(p.events().is_empty());
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn wasm_first_native_counterexample_requires_grammar_review_in_next() {
    assert!(
        std::env::var("CODEGUARD_RUST_RESOLUTION_TOOL").is_err(),
        "controlled counterexample only"
    );
    let p = Project::new_with_origin(true);
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("probe-mode"), "zero").unwrap();
    let review = p.verify(&tool, &policy).unwrap();
    assert_eq!(review["state"], "verification_required");
    assert_eq!(review["outcome"], "false_positive_review_required");
    let evidence: Value = serde_json::from_slice(
        &fs::read(p.root.join(review["evidence_ref"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(evidence["schema_version"], "0.9.0");
    assert_eq!(evidence["grammar_sha256"], policy["grammar_sha256"]);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.root)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&out.stdout).unwrap();
    let brief = codeguard_cli::next_command::read_task_brief(&p.root, &p.id).unwrap();
    assert_eq!(brief["disposition"], "verification_required");
    assert!(
        brief["step"].as_str().unwrap().contains("首次 WASM"),
        "{brief}"
    );
    assert!(
        brief["step"]
            .as_str()
            .unwrap()
            .contains("不直接认定 grammar 缺陷"),
        "{brief}"
    );
    assert_eq!(next["repair_brief"]["task_id"], p.id, "{next}");
    assert!(
        !p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "resolved")
    );
}
#[cfg(feature = "wasm-precheck")]
#[test]
#[ignore = "需要已安装直接Rustfmt1.9.0-stable；信任根仍为测试宿主夹具"]
fn real_rustfmt_wasm_first_resolution_and_recurrence() {
    assert!(std::env::var("CODEGUARD_RUST_RESOLUTION_TOOL").is_ok());
    wasm_first_rust_task_confirms_resolves_and_reopens_with_original_grammar();
}
