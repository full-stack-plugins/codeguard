#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

struct Project(PathBuf);
impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-syntax-tasks-{label}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), "pub fn main( void {\n").unwrap();
        let p = Self(root);
        let o = p
            .command()
            .args(["init"])
            .arg(&p.0)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        p
    }
    fn command(&self) -> Command {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
    }
    fn hook(&self) -> Value {
        self.hook_paths(&["app.zig"])
    }
    fn hook_paths(&self, paths: &[&str]) -> Value {
        let mut c = self.command();
        c.args(["hook", "execute"])
            .arg(&self.0)
            .args(["--timeout=30s", "--format=json"])
            .env("PATH", &self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        let request = json!({"schema_version":"1.0.0", "report_type":"hook_trigger_request", "input":{"event":"file_changed", "changed_paths":paths, "task_id":null, "write_outcome":"confirmed", "host_claims_blocking":false}});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(request.to_string().as_bytes())
            .unwrap();
        let o = child.wait_with_output().unwrap();
        assert_eq!(
            o.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recovery_tasks_are_stable_and_clean_candidates_cannot_close_them() {
    let p = Project::new("stable");
    let a = p.hook();
    assert!(
        a["local_feedback"]["candidate_recovery_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    let tasks = &a["local_feedback"]["syntax_tasks"];
    assert_eq!(tasks["status"], "synced_partial", "{a}");
    let id = tasks["tasks"][0]["task_id"].as_str().unwrap();
    let b = p.hook();
    assert_eq!(
        b["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    assert_eq!(b["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
    let o = p
        .command()
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(next["repair_brief"]["task_id"], id, "{next}");
    assert_eq!(
        next["repair_brief"]["checker_id"],
        "syntax.native_confirmation"
    );
    assert!(
        !next["repair_brief"]["recheck_argv"]
            .to_string()
            .contains("python")
    );
    fs::write(p.0.join("app.zig"), "pub fn main() void {}\n").unwrap();
    let c = p.hook();
    assert_eq!(c["local_feedback"]["candidate_recovery_count"], 0);
    assert!(
        c["local_feedback"]["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let o = p
        .command()
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let verify: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        verify["native_scan"]["native"]["reason"], "explicit_zig_tool_not_provided",
        "{verify}"
    );
}

#[test]
fn failed_persistence_keeps_recovery_evidence_without_fake_task_ids() {
    let p = Project::new("failed");
    fs::remove_dir(p.0.join(".codeguard/reports")).unwrap();
    fs::write(p.0.join(".codeguard/reports"), "occupied").unwrap();
    let r = p.hook();
    assert!(
        r["local_feedback"]["candidate_recovery_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        r["local_feedback"]["syntax_tasks"]["status"], "incomplete",
        "{r}"
    );
    assert!(
        r["local_feedback"]["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn python_edit_reuses_existing_confirmation_identity() {
    let p = Project::new("python");
    fs::write(p.0.join("broken.py"), "def broken(\n").unwrap();
    let o = p
        .command()
        .args(["lint", "python"])
        .arg(&p.0)
        .args(["--file", "broken.py", "--format=json"])
        .env("PATH", &p.0)
        .output()
        .unwrap();
    let first: Value = serde_json::from_slice(&o.stdout).unwrap();
    let id = first["setup"]["task_id"].as_str().unwrap();
    let r = p.hook_paths(&["broken.py"]);
    assert_eq!(
        r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"], id,
        "{r}"
    );
    assert_eq!(r["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
}

#[test]
fn import_rejects_forged_grammar_coordinates_and_duplicate_keys() {
    let p = Project::new("forged");
    let r = p.hook();
    let id = r["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let dir = p.0.join(".codeguard/reports");
    let original = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("syntax-confirm-")
        })
        .unwrap();
    let base: Value = serde_json::from_slice(&fs::read(original).unwrap()).unwrap();
    for (i, field) in ["grammar_sha256", "start_byte", "duplicate"]
        .iter()
        .enumerate()
    {
        let mut fake = base.clone();
        let run = format!("syntax-confirm-1-{}", i + 1);
        fake["run_id"] = json!(run);
        match *field {
            "grammar_sha256" => fake["observations"][0]["grammar_sha256"] = json!("0".repeat(64)),
            "start_byte" => fake["observations"][0]["recoveries"][0]["start_byte"] = json!(99999),
            _ => (),
        }
        let mut encoded = fake.to_string();
        if *field == "duplicate" {
            encoded = encoded.replacen("{", "{\"coverage_proven\":true,", 1);
        }
        fs::write(dir.join(format!("{run}.json")), encoded).unwrap();
    }
    let o = p
        .command()
        .args(["work", "sync"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(result["failed_reports"], 3, "{result}");
    assert_eq!(result["new_blockers"], 0);
    assert_eq!(
        fs::read_dir(p.0.join(format!(".codeguard/findings/{id}/events")))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn javascript_typescript_and_tsx_confirmation_tasks_reuse_eslint_identity() {
    let p = Project::new("node-scopes");
    for (path, content) in [
        ("bad.js", "const value = ;\n"),
        ("bad.ts", "const value: number = ;\n"),
        ("bad.tsx", "const view = <div>\n"),
    ] {
        fs::write(p.0.join(path), content).unwrap();
    }
    let a = p.hook_paths(&["bad.js", "bad.ts", "bad.tsx"]);
    let tasks = a["local_feedback"]["syntax_tasks"]["tasks"]
        .as_array()
        .unwrap();
    assert_eq!(tasks.len(), 3, "{a}");
    let b = p.hook_paths(&["bad.js", "bad.ts", "bad.tsx"]);
    assert_eq!(b["local_feedback"]["syntax_tasks"]["tasks"], json!(tasks));
    assert_eq!(b["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
    for task in tasks {
        let id = task["task_id"].as_str().unwrap();
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["checker_id"], "node.eslint.preparation");
        let o = p
            .command()
            .args(["task", "show", id])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn claude_edit_context_contains_real_confirmation_task_and_bounded_guidance() {
    let p = Project::new("claude");
    let mut c = p.command();
    c.args(["hook", "claude", "post-tool-use"])
        .arg(&p.0)
        .args(["--timeout=30s", "--format=json"])
        .env("PATH", &p.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let event = json!({"hook_event_name":"PostToolUse", "cwd":p.0, "tool_name":"Edit", "tool_input":{"file_path":p.0.join("app.zig"), "old_string":"old", "new_string":"new"}, "tool_response":{"success":true}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0));
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    let context = r["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("原生确认任务 CG-B-"), "{context}");
    assert!(context.contains("codeguard task show"));
    assert!(context.chars().count() <= 1200);
}
