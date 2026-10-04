#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
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
fn fresh_confirmation_guidance_matches_the_implemented_adapter_before_recheck() {
    for (label, file, source, option, version) in [
        (
            "fresh-zig",
            "app.zig",
            "const broken = ;\n",
            "--zig-tool",
            "Zig 0.16.0",
        ),
        (
            "fresh-erlang",
            "app.erl",
            "-module(app).\nf( -> ok.\n",
            "--erl-tool",
            "OTP 28",
        ),
        (
            "fresh-swift",
            "app.swift",
            "func f(_ x: ) {}\n",
            "--swift-tool",
            "Apple Swift 6.4",
        ),
    ] {
        let p = Project::new(label);
        fs::remove_file(p.0.join("app.zig")).unwrap();
        fs::write(p.0.join(file), source).unwrap();
        let report = p.hook_paths(&[file]);
        let id = report["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{report}"));
        // 只读查询即使能发现本地同名工具，也不得执行探测或安装。
        for tool in ["zig", "erl", "swiftc"] {
            let path = p.0.join(tool);
            fs::write(
                &path,
                "#!/bin/sh\nprintf executed > checker-executed\nexit 1\n",
            )
            .unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        for command in [vec!["next"], vec!["task", "show", id]] {
            let output = p
                .command()
                .args(command)
                .arg(&p.0)
                .env("PATH", &p.0)
                .current_dir(&p.0)
                .arg("--format=json")
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(0));
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            let brief = if value["repair_brief"].is_object() {
                &value["repair_brief"]
            } else {
                &value["task"]
            };
            assert_eq!(brief["task_id"], id, "{value}");
            assert_eq!(brief["schema_version"], "0.7.0", "{value}");
            assert_eq!(brief["tool_readiness"], "not_evaluated", "{value}");
            assert_eq!(brief["native_adapter"]["tool_option"], option, "{value}");
            assert_eq!(brief["disposition"], "verification_required", "{value}");
            let step = brief["step"].as_str().unwrap();
            assert!(
                step.contains(version) && step.contains("已接入") && step.contains("原生确认前"),
                "{value}"
            );
            assert!(!step.contains("adapter 尚未接入"), "{value}");
            let argv = brief["recheck_argv"].as_array().unwrap();
            assert!(argv.iter().any(|v| v == option), "{value}");
            if value["operation"] == "task_show" {
                assert_eq!(value["next_actions"][0], brief["recheck_argv"], "{value}");
            }
            assert!(
                brief["native_confirmation_ref"].is_null(),
                "未执行原生复检不能伪造证据: {value}"
            );
        }
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        assert!(!p.0.join("checker-executed").exists());
    }
}

#[test]
fn fresh_unimplemented_confirmation_adapter_keeps_the_capability_decision() {
    let p = Project::new("fresh-java");
    fs::remove_file(p.0.join("app.zig")).unwrap();
    fs::write(p.0.join("Broken.java"), "class Broken { void f( }\n").unwrap();
    let report = p.hook_paths(&["Broken.java"]);
    let id = report["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{report}"));
    let output = p
        .command()
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    let brief = &next["repair_brief"];
    assert_eq!(brief["task_id"], id, "{next}");
    assert_eq!(brief["disposition"], "needs_decision", "{next}");
    assert!(
        brief["step"]
            .as_str()
            .unwrap()
            .contains("java 的原生语法确认 adapter"),
        "{next}"
    );
    assert!(
        !brief["recheck_argv"].to_string().contains("--zig-tool"),
        "{next}"
    );
    assert!(
        !brief["recheck_argv"].to_string().contains("--erl-tool"),
        "{next}"
    );
    assert!(
        !brief["recheck_argv"].to_string().contains("--swift-tool"),
        "{next}"
    );
    assert!(brief["native_confirmation_ref"].is_null(), "{next}");
}

#[test]
fn changed_original_candidate_cannot_supply_fresh_adapter_guidance() {
    let p = Project::new("fresh-changed-report");
    let report = p.hook();
    let id = report["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{report}"));
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    let original = p.0.join(format!(
        ".codeguard/reports/{}.json",
        fact["first_run_id"].as_str().unwrap()
    ));
    let mut bytes = fs::read(&original).unwrap();
    bytes.push(b' ');
    fs::write(original, bytes).unwrap();
    let output = p
        .command()
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3), "{next}");
    assert_eq!(next["command_status"], "incomplete", "{next}");
    assert_eq!(next["reason"], "consumed_marker_invalid", "{next}");
    assert!(next["repair_brief"].is_null(), "{next}");
    assert_eq!(next["delivery_decision"], "not_evaluated", "{next}");
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

#[test]
fn unlocated_recoveries_become_stable_environment_tasks_without_source_positions() {
    let p = Project::new("unlocated");
    fs::remove_file(p.0.join("app.zig")).unwrap();
    for (path, source) in [
        ("bad.swift", "func f(_ x: ) {}\n"),
        ("bad.kt", "fun f(x: ) = x\n"),
        ("unknown.kt", "object C { val value = 1 }\n"),
    ] {
        fs::write(p.0.join(path), source).unwrap();
    }
    let paths = ["bad.swift", "bad.kt", "unknown.kt"];
    let a = p.hook_paths(&paths);
    let local = &a["local_feedback"];
    assert_eq!(local["candidate_recovery_count"], 0, "{a}");
    assert_eq!(local["next_action"], "require_native_lint_confirmation");
    let tasks = local["syntax_tasks"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3, "{a}");
    assert_eq!(local["syntax_tasks"]["new_blockers"], 3);
    let b = p.hook_paths(&paths);
    assert_eq!(b["local_feedback"]["syntax_tasks"]["tasks"], json!(tasks));
    assert_eq!(b["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
    for task in tasks {
        let id = task["task_id"].as_str().unwrap();
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["kind"], "blocker");
        assert_eq!(
            fact["first_diagnostic_reason"],
            "syntax_recovery_incomplete"
        );
        assert_eq!(fact["state"], "open");
        let body = fs::read_to_string(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        for field in [
            "问题证据",
            "规则依据",
            "允许修改范围",
            "修复步骤",
            "复检命令",
            "历史尝试",
            "关闭条件",
            "无法定位",
            "不得修改源码",
        ] {
            assert!(body.contains(field), "{field}: {body}");
        }
        let show = p
            .command()
            .args(["task", "show", id])
            .arg(&p.0)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(show.status.code(), Some(0), "{show:?}");
        let brief: Value = serde_json::from_slice(&show.stdout).unwrap();
        let step = brief["task"]["step"].as_str().unwrap();
        assert!(
            step.contains("无法定位") && step.contains("不得修改源码"),
            "{brief}"
        );
        assert!(
            brief["task"]["native_diagnostic_positions"].is_null()
                || brief["task"]["native_diagnostic_positions"] == json!([])
        );
        if brief["task"]["affected_paths"][0]
            .as_str()
            .is_some_and(|path| path.ends_with(".kt"))
        {
            let path = brief["task"]["affected_paths"][0].as_str().unwrap();
            fs::write(p.0.join(path), "object Changed {}\n").unwrap();
            let changed = p
                .command()
                .args(["task", "show", id])
                .arg(&p.0)
                .arg("--format=json")
                .output()
                .unwrap();
            assert_eq!(changed.status.code(), Some(0));
            let stale: Value = serde_json::from_slice(&changed.stdout).unwrap();
            let step = stale["task"]["step"].as_str().unwrap();
            assert!(
                step.contains("无法定位") && step.contains("不得修改源码"),
                "{stale}"
            );
            assert_eq!(stale["task"]["native_diagnostic_positions"], json!([]));
        }
    }
}

#[test]
fn unlocated_task_retains_identity_when_a_visible_recovery_later_appears() {
    let p = Project::new("unlocated-transition");
    fs::write(p.0.join("bad.swift"), "func f(_ x: ) {}\n").unwrap();
    let first = p.hook_paths(&["bad.swift"]);
    let id = first["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .expect("unlocated task");
    let verify = p
        .command()
        .args(["task", "verify", id])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(
        report["native_scan"]["native"]["reason"], "explicit_swift_tool_not_provided",
        "{report}"
    );
    assert_eq!(report["observation"], "incomplete", "{report}");
    fs::write(p.0.join("bad.swift"), "func f() {\n").unwrap();
    let visible = p.hook_paths(&["bad.swift"]);
    assert!(
        visible["local_feedback"]["candidate_recovery_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        visible["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    assert_eq!(visible["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
    fs::write(p.0.join("bad.swift"), "func f(_ x: Int) {}\n").unwrap();
    let clean = p.hook_paths(&["bad.swift"]);
    assert_eq!(
        clean["local_feedback"]["next_action"],
        "recommend_native_lint"
    );
    assert_eq!(clean["local_feedback"]["syntax_tasks"]["tasks"], json!([]));
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn unlocated_import_rejects_clean_zero_counts_and_forged_identity() {
    let p = Project::new("unlocated-forged");
    fs::write(p.0.join("bad.swift"), "func f(_ x: ) {}\n").unwrap();
    let first = p.hook_paths(&["bad.swift"]);
    let id = first["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .expect("unlocated task");
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
    assert_eq!(base["schema_version"], "0.3.0");
    for (i, (pointer, value)) in [
        ("/observations/0/reason", Value::Null),
        ("/schema_version", json!("0.1.0")),
        ("/schema_version", json!("9.0.0")),
        ("/observations/0/source_sha256", json!("0".repeat(64))),
        ("/observations/0/grammar_sha256", json!("0".repeat(64))),
        ("/observations/0/grammar_qualified", json!(true)),
        ("/observations/0/recovery_count", json!(1)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut fake = base.clone();
        let run = format!("syntax-confirm-1-{}", i + 1);
        fake["run_id"] = json!(run);
        *fake.pointer_mut(pointer).unwrap() = value;
        fs::write(dir.join(format!("{run}.json")), fake.to_string()).unwrap();
    }
    let output = p
        .command()
        .args(["work", "sync"])
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["failed_reports"], 7, "{result}");
    assert_eq!(result["new_blockers"], 0);
    assert_eq!(
        fs::read_dir(p.0.join(format!(".codeguard/findings/{id}/events")))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn unlocated_persistence_failure_retains_observation_without_fake_task() {
    let p = Project::new("unlocated-save");
    fs::write(p.0.join("bad.swift"), "func f(_ x: ) {}\n").unwrap();
    fs::remove_dir(p.0.join(".codeguard/reports")).unwrap();
    fs::write(p.0.join(".codeguard/reports"), "occupied").unwrap();
    let result = p.hook_paths(&["bad.swift"]);
    assert_eq!(
        result["local_feedback"]["syntax_candidates"]["observations"][0]["reason"],
        "syntax_recovery_incomplete"
    );
    assert_eq!(
        result["local_feedback"]["syntax_tasks"]["status"],
        "incomplete"
    );
    assert_eq!(result["local_feedback"]["syntax_tasks"]["tasks"], json!([]));
    assert_eq!(
        result["local_feedback"]["syntax_tasks"]["failures"][0]["reason"],
        "reports_directory_unavailable"
    );
}

#[test]
fn unlocated_project_check_and_edit_hook_share_the_same_task_and_next_step() {
    let p = Project::new("unlocated-check");
    fs::remove_file(p.0.join("app.zig")).unwrap();
    fs::write(p.0.join("bad.swift"), "func f(_ x: ) {}\n").unwrap();
    let output = p
        .command()
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--format=json", "--timeout", "30s"])
        .env("PATH", &p.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["syntax_tasks"]["status"], "synced_partial",
        "{report}"
    );
    assert_eq!(report["schema_version"], "0.40.0");
    assert_eq!(report["next"]["schema_version"], "0.7.0");
    let id = report["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    assert_eq!(report["next"]["repair_brief"]["task_id"], id);
    let hook = p.hook_paths(&["bad.swift"]);
    assert_eq!(
        hook["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    assert_eq!(hook["local_feedback"]["syntax_tasks"]["new_blockers"], 0);
    let human = p
        .command()
        .args(["check", "all"])
        .arg(&p.0)
        .args(["--timeout", "30s"])
        .env("PATH", &p.0)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&human.stdout).contains(id),
        "{human:?}"
    );
}

#[test]
fn unlocated_claude_context_explains_zero_positions_and_real_recovery_task() {
    let p = Project::new("unlocated-context");
    fs::write(p.0.join("bad.swift"), "func f(_ x: ) {}\n").unwrap();
    let mut c = p.command();
    c.args(["hook", "claude", "post-tool-use"])
        .arg(&p.0)
        .args(["--timeout=30s", "--format=json"])
        .env("PATH", &p.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let event = json!({"hook_event_name":"PostToolUse", "cwd":p.0, "tool_name":"Edit", "tool_input":{"file_path":p.0.join("bad.swift"), "old_string":"old", "new_string":"new"}, "tool_response":{"success":true}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let context = report["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        context.contains("疑似恢复节点 0 项") && context.contains("恢复扫描未完成 1 项"),
        "{context}"
    );
    assert!(
        context.contains("原生确认任务 CG-B-") && context.contains("codeguard task show"),
        "{context}"
    );
    assert!(!context.contains("建议安装适用原生 lint"), "{context}");
    assert!(context.chars().count() <= 1200);
}
