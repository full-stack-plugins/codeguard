#![cfg(unix)]
use serde_json::{Value, json};
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
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-shell-work-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("app.sh"), "#!/bin/bash\necho $1\n").unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&p)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        Self(p)
    }
    fn lint(&self, present: bool) -> Value {
        let tool = self.0.join("shellcheck");
        let diagnostic = json!({"comments":[{"file":"-","line":2,"endLine":2,"column":6,"endColumn":8,"level":"info","code":2086,"message":"private","fix":null}]});
        let output = if present {
            diagnostic.to_string()
        } else {
            "{\"comments\":[]}".into()
        };
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: 0.11.0\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '%s\\n' '{}'; exit {}\n",output,if present {1}else{0})).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        self.run(&[
            "lint",
            "shell",
            self.0.join("app.sh").to_str().unwrap(),
            "--dialect",
            "bash",
            "--shellcheck-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
    }
    fn run(&self, args: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", &self.0)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(if args[0] == "next" { 0 } else { 3 }),
            "{out:?}"
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn tasks(&self) -> Vec<String> {
        fs::read_dir(self.0.join(".codeguard/tasks"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|p| p.ends_with(".md"))
            .collect()
    }
}
#[test]
fn same_shell_rule_group_updates_one_task_and_zero_diagnostics_never_closes_it() {
    let p = Project::new();
    let first = p.lint(true);
    assert_eq!(first["workbench"]["status"], "synced_partial");
    let ids = p.tasks();
    assert_eq!(ids.len(), 1);
    let projection = fs::read_to_string(p.0.join(".codeguard/tasks").join(&ids[0])).unwrap();
    assert!(projection.contains(&format!("task verify {}", ids[0].trim_end_matches(".md"))));

    let second = p.lint(true);
    assert_eq!(
        second["workbench"]["task_ids"],
        first["workbench"]["task_ids"]
    );
    assert_eq!(p.tasks(), ids);
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(next["schema_version"], "0.17.0");
    assert_eq!(next["repair_brief"]["recheck_argv"][1], "task");
    assert_eq!(next["repair_brief"]["recheck_argv"][2], "verify");
    assert_eq!(
        next["repair_brief"]["recheck_argv"][3],
        first["workbench"]["task_ids"][0]
    );
    assert_eq!(
        next["repair_brief"]["recheck_argv"][4],
        p.0.to_str().unwrap()
    );
    let shown = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "show",
            first["workbench"]["task_ids"][0].as_str().unwrap(),
            p.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(shown.status.code(), Some(0));
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(
        shown["next_actions"][0],
        next["repair_brief"]["recheck_argv"]
    );
    let text = next.to_string();
    assert!(text.contains("shell.shellcheck"));
    assert!(text.contains("SC2086"));
    assert!(text.contains("shellcheck"));
    let clean = p.lint(false);
    assert_eq!(clean["workbench"]["status"], "synced_partial");
    assert_eq!(p.tasks(), ids);
    let id = ids[0].trim_end_matches(".md");
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn malformed_shell_workbench_report_cannot_create_findings() {
    let p = Project::new();
    p.lint(true);
    let reports = p.0.join(".codeguard/reports");
    let path = fs::read_dir(&reports)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut packet: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    packet["run_id"] = json!("shellcheck-forged");
    packet["native"]["diagnostics"][0]["column"] = json!(0);
    fs::write(
        reports.join("shellcheck-forged.json"),
        serde_json::to_vec(&packet).unwrap(),
    )
    .unwrap();
    let out = p.run(&["work", "sync", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(out["failed_reports"], 1);
    assert_eq!(p.tasks().len(), 1);
}

#[test]
fn changed_rc_withdraws_actionable_old_positions_and_blockers_keep_one_task() {
    let p = Project::new();
    p.lint(true);
    fs::write(p.0.join(".shellcheckrc"), "disable=SC2086\n").unwrap();
    p.lint(false);
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(next["disposition"], "verification_required");
    let q = Project::new();
    fs::write(q.0.join(".shellcheckrc"), "external-sources=true\n").unwrap();
    let first = q.lint(true);
    assert_eq!(first["workbench"]["status"], "synced_partial");
    let ids = q.tasks();
    assert_eq!(ids.len(), 1);
    assert!(ids[0].starts_with("CG-B-"));
    fs::write(q.0.join(".shellcheckrc"), [0xff]).unwrap();
    let second = q.lint(true);
    assert_eq!(
        second["workbench"]["task_ids"],
        first["workbench"]["task_ids"]
    );
    assert_eq!(q.tasks(), ids);
}

#[test]
fn task_verify_uses_original_shell_rule_and_preserves_suppression_review() {
    let p = Project::new();
    let first = p.lint(true);
    let id = first["workbench"]["task_ids"][0].as_str().unwrap();
    let tool = p.0.join("shellcheck");
    let args = [
        "task",
        "verify",
        id,
        p.0.to_str().unwrap(),
        "--shellcheck-tool",
        tool.to_str().unwrap(),
        "--format=json",
    ];
    let observed = p.run(&args);
    assert_eq!(observed["observation"], "still_present");
    assert_eq!(observed["event_persisted"], true);
    fs::write(p.0.join(".shellcheckrc"), "disable=SC2086\n").unwrap();
    p.lint(false);
    let suppressed = p.run(&args);
    assert_eq!(
        suppressed["observation"], "rule_coverage_requires_review",
        "{suppressed}"
    );
    assert_eq!(suppressed["event_persisted"], true);
    fs::remove_file(p.0.join(".shellcheckrc")).unwrap();
    fs::write(p.0.join("app.sh"), "#!/bin/bash\necho \"$1\"\n").unwrap();
    let clean = p.run(&args);
    assert_eq!(clean["observation"], "candidate_absent_unverified_policy");
    assert_eq!(clean["event_persisted"], true);
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(next["repair_brief"]["checker_id"], "shell.shellcheck");
}

#[test]
fn shell_verify_rejects_foreign_tool_flags_before_claiming_or_running() {
    let p = Project::new();
    let first = p.lint(true);
    let id = first["workbench"]["task_ids"][0].as_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            p.0.to_str().unwrap(),
            "--ruff-tool",
            p.0.join("shellcheck").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn shell_recheck_cannot_import_a_rule_absent_from_the_original_report() {
    use sha2::{Digest, Sha256};
    let p = Project::new();
    let first = p.lint(true);
    let id = first["workbench"]["task_ids"][0].as_str().unwrap();
    let report = p.run(&[
        "task",
        "verify",
        id,
        p.0.to_str().unwrap(),
        "--shellcheck-tool",
        p.0.join("shellcheck").to_str().unwrap(),
        "--format=json",
    ]);
    let mut scan = report["native_scan"].clone();
    scan["run_id"] = json!("shellcheck-forged-rule");
    scan["task_rule"] = json!("SC2000");
    let mut hash = Sha256::new();
    for text in [
        "codeguard-shell-rule-group-v1",
        "shell.shellcheck",
        "app.sh",
        "bash",
        "SC2000",
    ] {
        hash.update((text.len() as u64).to_be_bytes());
        hash.update(text.as_bytes());
    }
    scan["task_id"] = json!(format!("CG-{}", &format!("{:x}", hash.finalize())[..32]));
    fs::write(
        p.0.join(".codeguard/reports/shellcheck-forged-rule.json"),
        serde_json::to_vec(&scan).unwrap(),
    )
    .unwrap();
    let sync = p.run(&["work", "sync", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(sync["failed_reports"], 1, "{sync}");
    assert_eq!(p.tasks().len(), 1);
}

#[test]
fn failed_original_shell_rechecks_bind_attempts_and_exhaust_the_same_action() {
    let p = Project::new();
    let first = p.lint(true);
    let id = first["workbench"]["task_ids"][0].as_str().unwrap();
    let invoke = |args: &[&str], expected: i32| -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", &p.0)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(expected), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let lease = invoke(
        &[
            "task",
            "claim",
            id,
            p.0.to_str().unwrap(),
            "--owner",
            "agent-shell",
            "--format=json",
        ],
        0,
    );
    let token = lease["lease_token"].as_str().unwrap();
    let mut action = String::new();
    for _ in 0..2 {
        let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"]);
        action = next["repair_brief"]["action_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let start = invoke(
            &[
                "task",
                "attempt",
                "start",
                id,
                p.0.to_str().unwrap(),
                "--owner",
                "agent-shell",
                "--lease-token",
                token,
                "--action-id",
                &action,
                "--format=json",
            ],
            0,
        );
        let attempt = start["attempt_id"].as_str().unwrap();
        invoke(
            &[
                "task",
                "attempt",
                "finish",
                id,
                p.0.to_str().unwrap(),
                "--owner",
                "agent-shell",
                "--lease-token",
                token,
                "--attempt-id",
                attempt,
                "--outcome",
                "ready-to-verify",
                "--note-code",
                "source_edit",
                "--format=json",
            ],
            0,
        );
        let result = invoke(
            &[
                "task",
                "verify",
                id,
                p.0.to_str().unwrap(),
                "--owner",
                "agent-shell",
                "--lease-token",
                token,
                "--shellcheck-tool",
                p.0.join("shellcheck").to_str().unwrap(),
                "--format=json",
            ],
            3,
        );
        assert_eq!(result["observation"], "still_present");
        assert_eq!(result["event_persisted"], true);
        let run = result["native_scan"]["run_id"].as_str().unwrap();
        let event: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/events/verify-{run}.json")))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(event["attempt_id"], attempt);
    }
    let next = p.run(&["next", p.0.to_str().unwrap(), "--format=json"]);
    assert_eq!(next["disposition"], "needs_decision", "{next}");
    assert_eq!(next["repair_brief"]["history"]["no_progress_count"], 2);
    let denied = invoke(
        &[
            "task",
            "attempt",
            "start",
            id,
            p.0.to_str().unwrap(),
            "--owner",
            "agent-shell",
            "--lease-token",
            token,
            "--action-id",
            &action,
            "--format=json",
        ],
        3,
    );
    assert_eq!(denied["reason"], "no_progress_budget_exhausted");
}
