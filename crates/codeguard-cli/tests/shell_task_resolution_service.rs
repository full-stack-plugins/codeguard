#![cfg(unix)]
use codeguard_cli::{
    ApprovalTrustKey, ApprovalVerificationContext, ShellTaskResolutionRequest,
    verify_shell_task_resolution,
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
const BAD: &[u8] = b"#!/bin/bash\necho $1\n";
const GOOD: &[u8] = b"#!/bin/bash\necho \"$1\"\n";
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
const NORMAL_TOOL: &str = "input=$(/bin/cat)\ncase \"$input\" in *'echo $1'*) printf '%s' '{\"comments\":[{\"file\":\"-\",\"line\":2,\"endLine\":2,\"column\":6,\"endColumn\":8,\"level\":\"info\",\"code\":2086,\"message\":\"private\",\"fix\":null}]}'; exit 1;; *) printf '%s' '{\"comments\":[]}';; esac";
fn write_tool(root: &Path, body: &str) -> PathBuf {
    let p = root.join("shellcheck");
    fs::write(&p,format!("#!/bin/sh\nprintf x >> \"$0.calls\"\nif [ \"$1\" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: 0.11.0\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    p
}
struct Project {
    root: PathBuf,
    id: String,
    workspace: String,
    original_report: String,
    configuration: Value,
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Project {
    fn new() -> Self {
        Self::create_with_tool(None)
    }
    fn create_with_tool(native_tool: Option<&Path>) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-shell-resolution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.sh"), BAD).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let tool = native_tool
            .map(Path::to_path_buf)
            .unwrap_or_else(|| write_tool(&root, NORMAL_TOOL));
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "shell"])
            .arg(root.join("app.sh"))
            .args(["--dialect", "bash", "--shellcheck-tool"])
            .arg(tool)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        let id = r["workbench"]["task_ids"][0].as_str().unwrap().to_owned();
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        Self {
            root,
            id,
            workspace: fact["workspace_id"].as_str().unwrap().into(),
            original_report: fact["first_report_sha256"].as_str().unwrap().into(),
            configuration: r["project_configuration"].clone(),
        }
    }
    fn tool(&self, body: &str) -> PathBuf {
        write_tool(&self.root, body)
    }
    fn normal_tool(&self) -> PathBuf {
        self.tool(NORMAL_TOOL)
    }
    fn policy(&self, tool: &Path) -> Value {
        json!({"schema_version":"1.10.0","report_type":"task_resolution_policy","identity":{"workspace_id":self.workspace,"task_id":self.id,"checker_id":"shell.shellcheck","scope":"app.sh"},"policy_revision":"p1","original_report_sha256":self.original_report,"original_source_sha256":sha(BAD),"grammar_sha256":null,"tool_sha256":sha(&fs::read(tool).unwrap()),"adapter_sha256":sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"native_rule_id":"SC2086","native_version":"0.11.0","dialect":"bash","project_configuration":self.configuration})
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
        verify_shell_task_resolution(&ShellTaskResolutionRequest {
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
fn shell_original_rule_closes_idempotently_and_public_recheck_reopens() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.sh"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    export(&p, &policy, &r, "fixed");
    assert_eq!(
        p.verify(&tool, &policy).unwrap()["event_ref"],
        r["event_ref"]
    );
    fs::write(p.root.join("app.sh"), BAD).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", &p.id])
        .arg(&p.root)
        .arg("--shellcheck-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(
        p.events()
            .iter()
            .any(|e| e["event"]["kind"]["event"] == "reopened")
    );
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "open");
    export(&p, &policy, &r, "reopened");
}
#[test]
fn shell_config_or_disable_comment_cannot_be_claimed_as_code_fix() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    fs::write(
        p.root.join("app.sh"),
        b"#!/bin/bash\n# shellcheck disable=SC2086\necho \"$1\"\n",
    )
    .unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_ne!(r["state"], "resolved");
    export(&p, &policy, &r, "suppression");
    fs::write(p.root.join(".shellcheckrc"), "disable=SC2086\n").unwrap();
    assert!(p.verify(&tool, &policy).is_err());
}
#[test]
#[ignore = "requires existing explicitly selected ShellCheck 0.11.0; signing host remains a fixture"]
fn real_shellcheck_rule_fix_and_recurrence() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_SHELLCHECK_BIN").unwrap());
    let p = Project::create_with_tool(Some(&tool));
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.sh"), GOOD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    export(&p, &policy, &r, "real-fixed");
    fs::write(p.root.join("app.sh"), BAD).unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "open");
    export(&p, &policy, &r, "reopened");
}

#[test]
#[ignore = "requires existing explicitly selected ShellCheck 0.11.0; original-rule closure retains other diagnostics"]
fn real_shellcheck_other_rule_does_not_prevent_original_task_closure() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_SHELLCHECK_BIN").unwrap());
    let p = Project::create_with_tool(Some(&tool));
    let policy = p.policy(&tool);
    fs::write(p.root.join("app.sh"), b"#!/bin/bash\necho \"$missing\"\n").unwrap();
    let r = p.verify(&tool, &policy).unwrap();
    assert_eq!(r["state"], "resolved");
    export(&p, &policy, &r, "fixed");
    let evidence: Value = serde_json::from_slice(
        &fs::read(p.root.join(r["evidence_ref"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(evidence["current_native"]["status"], "diagnostics_observed");
    assert!(
        evidence["current_native"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule_id"] == "SC2154")
    );
    assert!(
        fs::read_dir(p.root.join(".codeguard/findings"))
            .unwrap()
            .any(|e| {
                let path = e.unwrap().path().join("finding.json");
                fs::read(path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                    .is_some_and(|fact| fact["native_rule_id"] == "SC2154")
            })
    );
}

#[test]
fn foreign_rule_policy_and_untrusted_host_do_not_run_the_tool() {
    let p = Project::new();
    let tool = p.normal_tool();
    let policy = p.policy(&tool);
    let calls = fs::read(p.root.join("shellcheck.calls")).unwrap();
    for (key, value) in [
        ("schema_version", "1.9.0"),
        ("native_rule_id", "SC2154"),
        ("native_version", "0.10.0"),
        ("dialect", "sh"),
        ("grammar_sha256", &"a".repeat(64)),
    ] {
        let mut bad = policy.clone();
        bad[key] = json!(value);
        assert!(p.verify(&tool, &bad).is_err());
    }
    let raw = serde_json::to_vec(&policy).unwrap();
    for host in [
        "wrong_key",
        "revoked",
        "expired",
        "clock_missing",
        "rollback",
    ] {
        assert!(p.verify_raw(&tool, &raw, None, host).is_err());
    }
    assert_eq!(fs::read(p.root.join("shellcheck.calls")).unwrap(), calls);
    assert!(p.events().is_empty());
}

fn export(p: &Project, policy: &Value, receipt: &Value, label: &str) {
    if let Ok(path) = std::env::var("CODEGUARD_SHELL_RESOLUTION_REPORT_DIR") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path).unwrap();
        let evidence: Value = serde_json::from_slice(
            &fs::read(p.root.join(receipt["evidence_ref"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let row = json!({"policy":policy,"receipt":receipt,"evidence":evidence});
        fs::write(
            path.join(format!(
                "{label}-{}-{}.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn shell_resolution_consumes_attempt_without_releasing_borrowed_lease() {
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
    command(&["task", "verify", &p.id, "--shellcheck-tool", tool_arg]);
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
    fs::write(p.root.join("app.sh"), GOOD).unwrap();
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
