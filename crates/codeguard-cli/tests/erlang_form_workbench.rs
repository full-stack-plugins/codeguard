#![cfg(all(feature = "wasm-precheck", unix))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(root: &std::path::Path, args: &[&str], input: Option<Value>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    if args[0] == "hook" {
        let boundary = if args[1] == "claude" { 3 } else { 2 };
        command
            .args(&args[..boundary])
            .arg(root)
            .args(&args[boundary..]);
    } else {
        command.args(args).arg(root);
    }
    let mut child = command
        .arg("--format=json")
        .env("PATH", "/no/tools")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().unwrap();
    assert!(matches!(output.status.code(), Some(0 | 3)), "{output:?}");
    let value = serde_json::from_slice(&output.stdout).unwrap();
    if let Some(dir) = std::env::var_os("CODEGUARD_ERLANG_WORKBENCH_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        static BINARY_SHA: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        let binary_sha = BINARY_SHA.get_or_init(|| {
            use sha2::{Digest, Sha256};
            format!(
                "{:x}",
                Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
            )
        });
        fs::write(PathBuf::from(&dir).join("binary.sha256"), binary_sha).unwrap();
        fs::write(
            PathBuf::from(dir).join(format!("{count}.json")),
            &output.stdout,
        )
        .unwrap();
    }
    value
}
#[test]
fn project_and_edit_checks_reuse_one_task_and_clean_candidate_cannot_close_it() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-erlang-form-workbench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("input.erl"), "-module(sample).\nf() -> ok\n").unwrap();
    run(&fixture.0, &["init", "--apply"], None);
    let first = run(&fixture.0, &["check", "all"], None);
    let row = first["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["language"] == "erlang")
        .unwrap();
    assert_eq!(row["recovery_count"], 0);
    assert_eq!(row["structural_observation_count"], 1, "{first}");
    assert_eq!(
        row["structural_observations"][0]["rule_id"],
        "codeguard.erlang.form_terminator"
    );
    assert_eq!(first["syntax_tasks"]["failures"], json!([]));
    let id = first["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let second = run(&fixture.0, &["check", "all"], None);
    assert_eq!(second["syntax_tasks"]["tasks"][0]["task_id"], id);
    assert_eq!(second["syntax_tasks"]["new_blockers"], 0);
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let hook = run(
        &fixture.0,
        &["hook", "execute", "--timeout", "30s"],
        Some(event),
    );
    assert_eq!(
        hook["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"], id,
        "{hook}"
    );
    assert_eq!(
        hook["local_feedback"]["syntax_candidates"]["observations"][0]["structural_observation_count"],
        1
    );
    let file = fixture.0.join("input.erl");
    let claude = run(
        &fixture.0,
        &["hook", "claude", "post-tool-use", "--timeout", "30s"],
        Some(json!({
            "session_id":"erlang-fixture", "cwd":fixture.0, "hook_event_name":"PostToolUse", "tool_name":"Write",
            "tool_input":{"file_path":file,"content":"HOST_SOURCE_MUST_NOT_BE_ECHOED"},
            "tool_response":{"filePath":file,"type":"create"}
        })),
    );
    let context = claude["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(
        context.contains("codeguard.erlang.form_terminator"),
        "{context}"
    );
    assert!(!context.contains("HOST_SOURCE_MUST_NOT_BE_ECHOED"));
    let task =
        fs::read_to_string(fixture.0.join(".codeguard/tasks").join(format!("{id}.md"))).unwrap();
    assert!(task.contains("Erlang"), "{task}");
    assert!(
        task.contains("规则依据") && task.contains("历史尝试") && task.contains("关闭条件"),
        "{task}"
    );
    let reports = fixture.0.join(".codeguard/reports");
    let original: Value = fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter_map(|p| serde_json::from_slice::<Value>(&fs::read(p).unwrap()).ok())
        .find(|v| v["schema_version"] == "0.14.0")
        .unwrap();
    if let Some(dir) = std::env::var_os("CODEGUARD_ERLANG_WORKBENCH_REPORT_DIR") {
        fs::write(
            PathBuf::from(dir).join("confirmation.json"),
            serde_json::to_vec_pretty(&original).unwrap(),
        )
        .unwrap();
    }
    for index in 0..4 {
        let mut forged = original.clone();
        forged["run_id"] = json!(format!("syntax-confirm-999-{}", 1000 + index));
        match index {
            0 => forged["observations"][0]["grammar_sha256"] = json!("0".repeat(64)),
            1 => {
                forged["observations"][0]["structural_observations"][0]["rule_sha256"] =
                    json!("0".repeat(64))
            }
            2 => forged["observations"][0]["structural_observations"][0]["end_byte"] = json!(99999),
            _ => forged["schema_version"] = json!("0.7.0"),
        }
        fs::write(
            reports.join(format!("{}.json", forged["run_id"].as_str().unwrap())),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
    }
    let sync = run(&fixture.0, &["work", "sync"], None);
    assert_eq!(sync["failed_reports"], 4, "{sync}");
    assert_eq!(sync["new_findings"], 0);
    assert_eq!(sync["new_blockers"], 0);
    for index in 0..4 {
        fs::remove_file(reports.join(format!("syntax-confirm-999-{}.json", 1000 + index))).unwrap();
    }
    fs::write(
        fixture.0.join("input.erl"),
        "-module(sample).\nf() -> ok.\n",
    )
    .unwrap();
    let clean = run(&fixture.0, &["check", "all"], None);
    assert_eq!(clean["syntax_tasks"]["tasks"], json!([]));
    let next = run(&fixture.0, &["next"], None);
    assert!(next.to_string().contains(id), "{next}");
}

#[test]
fn hook_first_structure_task_retains_evidence_after_source_changes_and_native_failure() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-erlang-hook-first-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(
        fixture.0.join("input.erl"),
        "-module(sample).\nf() -> ok;\n",
    )
    .unwrap();
    run(&fixture.0, &["init", "--apply"], None);
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let hook = run(
        &fixture.0,
        &["hook", "execute", "--timeout", "30s"],
        Some(event),
    );
    let id = hook["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let task =
        fs::read_to_string(fixture.0.join(".codeguard/tasks").join(format!("{id}.md"))).unwrap();
    assert!(
        task.contains("函数终止符") && task.contains("结构") && task.contains("--erl-tool"),
        "{task}"
    );
    let show = run(&fixture.0, &["task", "show", id], None);
    assert!(show.to_string().contains("--erl-tool"), "{show}");
    fs::write(
        fixture.0.join("input.erl"),
        "-module(sample).\nf() -> ok.\n",
    )
    .unwrap();
    let show = run(&fixture.0, &["task", "show", id], None);
    assert!(show.to_string().contains("--erl-tool"), "{show}");
    let verified = run(&fixture.0, &["task", "verify", id], None);
    assert_eq!(
        verified["native_scan"]["native"]["status"], "not_run",
        "{verified}"
    );
    assert!(
        verified.to_string().contains("erlang_tool_not_found"),
        "{verified}"
    );
    let clean = run(&fixture.0, &["check", "all"], None);
    assert_eq!(clean["syntax_tasks"]["tasks"], json!([]));
    let show = run(&fixture.0, &["task", "show", id], None);
    assert!(
        show.to_string().contains(id) && show.to_string().contains("open"),
        "{show}"
    );
}

#[test]
fn explicit_bad_erlang_tool_keeps_native_failure_without_wasm_fallback() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-erlang-bad-native-hook-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("input.erl"), "-module(sample).\nf()->ok;\n").unwrap();
    let tool = fixture.0.join("wrong-erl");
    fs::write(&tool, "#!/bin/sh\nprintf '27'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    run(&fixture.0, &["init", "--apply"], None);
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let hook = run(
        &fixture.0,
        &[
            "hook",
            "execute",
            "--timeout",
            "30s",
            "--erl-tool",
            tool.to_str().unwrap(),
        ],
        Some(event),
    );
    let feedback = &hook["local_feedback"];
    assert_eq!(
        feedback["erlang_lint"]["tool_selection"]["source"], "explicit",
        "{hook}"
    );
    assert_eq!(
        feedback["erlang_lint"]["files"][0]["native"]["status"], "incomplete",
        "{hook}"
    );
    assert!(
        feedback["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{hook}"
    );
    assert_eq!(feedback["coverage_proven"], false);
    for key in ["kotlin_lint", "swift_lint", "zig_lint"] {
        assert!(feedback.get(key).is_some(), "{hook}");
    }
}

#[test]
#[ignore = "requires existing OTP 28 via CODEGUARD_ERL_BIN"]
fn actual_otp_edit_prefers_native_and_rechecks_the_same_task_after_fix() {
    let tool = std::env::var("CODEGUARD_ERL_BIN").unwrap();
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-erlang-actual-edit-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(
        fixture.0.join("input.erl"),
        "-module(sample).\nf() -> ok;\n",
    )
    .unwrap();
    run(&fixture.0, &["init", "--apply"], None);
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.erl"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let first = run(
        &fixture.0,
        &["hook", "execute", "--timeout", "30s", "--erl-tool", &tool],
        Some(event.clone()),
    );
    let file = &first["local_feedback"]["erlang_lint"]["files"][0];
    assert_eq!(file["native"]["status"], "diagnostics_observed", "{first}");
    let id = file["task_id"].as_str().unwrap();
    assert_eq!(
        first["local_feedback"]["next_action"],
        "repair_native_source"
    );
    assert!(
        first["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let bad = run(
        &fixture.0,
        &["task", "verify", id, "--erl-tool", &tool],
        None,
    );
    assert_eq!(
        bad["native_scan"]["native"]["status"], "diagnostics_observed",
        "{bad}"
    );
    fs::write(
        fixture.0.join("input.erl"),
        "-module(sample).\nf() -> ok.\n",
    )
    .unwrap();
    let fixed = run(
        &fixture.0,
        &["hook", "execute", "--timeout", "30s", "--erl-tool", &tool],
        Some(event),
    );
    assert_eq!(
        fixed["local_feedback"]["erlang_lint"]["files"][0]["task_id"], id,
        "{fixed}"
    );
    assert_eq!(
        fixed["local_feedback"]["erlang_lint"]["files"][0]["native"]["status"], "completed",
        "{fixed}"
    );
    assert!(
        fixed["local_feedback"]["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let good = run(
        &fixture.0,
        &["task", "verify", id, "--erl-tool", &tool],
        None,
    );
    assert_eq!(
        good["native_scan"]["native"]["status"], "completed",
        "{good}"
    );
    let show = run(&fixture.0, &["task", "show", id], None);
    assert!(show.to_string().contains("open"), "{show}");
}
