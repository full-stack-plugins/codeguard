#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
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
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-swift-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("App.swift"), "func f(_ x: ) {}\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("swiftc");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'Apple Swift version {version} (swiftlang-6.4)\\nTarget: arm64-apple-macosx26.0\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '<stdin>:1:13: error: expected type\\nfunc f(_ x: ) {{}}\\n            ^\\n' >&2\nexit 1\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, explicit: Option<&PathBuf>) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env("PATH", &self.0)
            .args(["check", "all"])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(p) = explicit {
            c.arg("--swift-tool").arg(p);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[test]
fn first_swift_native_scan_creates_one_task_and_rechecks_without_auto_close() {
    let p = Project::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init"])
        .arg(&p.0)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = p.tool("6.4");
    let r = p.check(Some(&tool));
    let scan = &r["native_results"]["swift_lint"];
    let id = scan["files"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    assert_eq!(scan["task_status"], "synced_partial");
    let again = p.check(Some(&tool));
    assert_eq!(
        again["native_results"]["swift_lint"]["files"][0]["task_id"],
        id
    );
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(next["repair_brief"]["action_id"], "repair-source", "{next}");
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(&id)
        .arg(&p.0)
        .args(["--swift-tool"])
        .arg(&tool)
        .arg("--format=json")
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let verify: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verify["native_scan"]["schema_version"], "0.7.0", "{verify}");
    assert_eq!(
        verify["native_scan"]["original_report"]["grammar_sha256"],
        Value::Null
    );
    assert_eq!(verify["observation"], "still_blocked");
    assert_eq!(verify["event_persisted"], true);
    let auto = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(&id)
        .arg(&p.0)
        .arg("--format=json")
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let auto: Value = serde_json::from_slice(&auto.stdout).unwrap();
    assert_eq!(
        auto["native_scan"]["native"]["status"], "diagnostics_observed",
        "{auto}"
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn clean_swift_first_observation_creates_no_task_but_existing_task_keeps_history() {
    let p = Project::new();
    let tool = p.tool("6.4");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let first = p.check(Some(&tool));
    let id = first["native_results"]["swift_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::write(p.0.join("App.swift"), "struct X {}\n").unwrap();
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace("printf '<stdin>", "exit 0\nprintf '<stdin>"),
    )
    .unwrap();
    let clean = p.check(Some(&tool));
    assert_eq!(
        clean["native_results"]["swift_lint"]["files"][0]["task_id"],
        id
    );
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(&id)
        .arg(&p.0)
        .arg("--swift-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["observation"], "candidate_absent_unverified_policy");
    assert_eq!(r["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let q = Project::new();
    fs::write(q.0.join("App.swift"), "struct X {}\n").unwrap();
    let tool = q.tool("6.4");
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace("printf '<stdin>", "exit 0\nprintf '<stdin>"),
    )
    .unwrap();
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&q.0)
        .arg("--apply")
        .output()
        .unwrap();
    let r = q.check(Some(&tool));
    assert!(r["native_results"]["swift_lint"]["files"][0]["task_id"].is_null());
}
