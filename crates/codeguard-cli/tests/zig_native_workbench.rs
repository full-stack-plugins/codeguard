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
            "cg-zig-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), "pub fn main( void {\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("zig");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = version ]; then printf '{version}\\n'; exit 0; fi\nIFS= read -r line || :\ncase \"$line\" in *'const Empty'*) exit 0;; esac\nprintf '<stdin>:1:13: error: failure\\n' >&2\nexit 1\n")).unwrap();
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
            c.arg("--zig-tool").arg(p);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn first_zig_native_scan_creates_one_task_and_current_original_tool_guidance() {
    let p = Project::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init"])
        .arg(&p.0)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = p.tool("0.16.0");
    let r = p.check(Some(&tool));
    let id = r["native_results"]["zig_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    assert_eq!(
        r["native_results"]["zig_lint"]["task_status"],
        "synced_partial"
    );
    let again = p.check(Some(&tool));
    assert_eq!(
        again["native_results"]["zig_lint"]["files"][0]["task_id"],
        id
    );
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["repair_brief"]["task_id"], id, "{next}");
    assert_eq!(next["repair_brief"]["action_id"], "repair-source", "{next}");
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("Zig AST")
    );
    assert!(
        next["repair_brief"]["recheck_argv"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "--zig-tool")
    );
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(&id)
        .arg(&p.0)
        .arg("--format=json")
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let verify: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verify["native_scan"]["schema_version"], "0.8.0", "{verify}");
    assert!(verify["native_scan"]["original_report"]["grammar_sha256"].is_null());
    assert_eq!(verify["observation"], "still_blocked");
    assert_eq!(verify["event_persisted"], true);
    fs::write(p.0.join("app.zig"), "const Empty = struct {};\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(&id)
        .arg(&p.0)
        .arg("--format=json")
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let clean: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        clean["observation"], "candidate_absent_unverified_policy",
        "{clean}"
    );
    assert_eq!(clean["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn clean_native_first_zig_creates_no_repair_task() {
    let p = Project::new();
    fs::write(p.0.join("app.zig"), "const Empty = struct {};\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let tool = p.tool("0.16.0");
    let r = p.check(Some(&tool));
    assert_eq!(
        r["native_results"]["zig_lint"]["task_status"], "synced_partial",
        "{r}"
    );
    assert!(r["native_results"]["zig_lint"]["files"][0]["task_id"].is_null());
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/findings"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn single_file_and_edit_hook_share_the_same_first_native_task() {
    use std::io::Write;
    use std::process::Stdio;
    let p = Project::new();
    let tool = p.tool("0.16.0");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "zig"])
        .arg(p.0.join("app.zig"))
        .arg("--zig-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let lint: Value = serde_json::from_slice(&o.stdout).unwrap();
    let id = lint["task_id"].as_str().unwrap_or_else(|| panic!("{lint}"));
    assert_eq!(lint["schema_version"], "0.3.0");
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["hook", "execute"])
        .arg(&p.0)
        .args(["--format=json", "--timeout=30s"])
        .env("PATH", &p.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request = serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.zig"],"write_outcome":"confirmed","host_claims_blocking":false}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    let hook: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        hook["local_feedback"]["zig_lint"]["files"][0]["task_id"], id,
        "{hook}"
    );
    assert_eq!(hook["schema_version"], "0.16.0");
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["hook", "execute"])
        .arg(&p.0)
        .args(["--format=json", "--timeout=30s"])
        .env("PATH", &p.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request = serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"repair_ready","changed_paths":[],"task_id":id,"write_outcome":"confirmed","host_claims_blocking":false}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    let hook: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(hook["schema_version"], "0.15.0", "{hook}");
    assert_eq!(
        hook["local_feedback"]["native_confirmation_status"],
        "diagnostics_observed"
    );
    assert!(!hook["local_feedback"]["native_confirmation_ref"].is_null());
}
#[test]
#[ignore = "requires existing explicitly supplied Zig 0.16.0; no installation"]
fn real_zig_first_native_task_rechecks_bad_and_fixed_input() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_ZIG_BIN").unwrap());
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&p.0)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "zig"])
        .arg(&p.0)
        .args(["--zig-tool"])
        .arg(&tool)
        .args(["--timeout", "120s", "--format=json"])
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let scan: Value = serde_json::from_slice(&o.stdout).unwrap();
    let id = scan["native_results"]["zig_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{scan}"));
    assert_eq!(
        scan["native_results"]["zig_lint"]["task_status"],
        "synced_partial"
    );
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(next["repair_brief"]["action_id"], "repair-source", "{next}");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(id)
        .arg(&p.0)
        .arg("--zig-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let broken: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(broken["observation"], "still_blocked", "{broken}");
    assert_eq!(broken["event_persisted"], true);
    assert!(broken["native_scan"]["original_report"]["grammar_sha256"].is_null());
    fs::write(p.0.join("app.zig"), "const Empty = struct {};\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify"])
        .arg(id)
        .arg(&p.0)
        .arg("--zig-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let fixed: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        fixed["observation"], "candidate_absent_unverified_policy",
        "{fixed}"
    );
    assert_eq!(fixed["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Ok(path) = std::env::var("CODEGUARD_ZIG_NATIVE_FIRST_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"first_scan":scan,"next":next,"broken":broken,"fixed":fixed,"fact_state":fact["state"]})).unwrap()).unwrap();
    }
}
