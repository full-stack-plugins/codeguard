#![cfg(feature = "wasm-precheck")]
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
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_WORKBENCH_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        let count = fs::read_dir(&dir).unwrap().count();
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
        .join(format!("cg-js-workbench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("input.js"), "const x=1; const x=2;\n").unwrap();
    run(&fixture.0, &["init", "--apply"], None);
    let first = run(&fixture.0, &["check", "all"], None);
    let row = first["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["language"] == "javascript")
        .unwrap();
    assert_eq!(row["recovery_count"], 0);
    assert_eq!(row["structural_observation_count"], 1, "{first}");
    assert_eq!(
        row["structural_observations"][0]["rule_id"],
        "codeguard.javascript.duplicate_direct_lexical_binding"
    );
    assert_eq!(first["syntax_tasks"]["failures"], json!([]));
    let id = first["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let second = run(&fixture.0, &["check", "all"], None);
    assert_eq!(second["syntax_tasks"]["tasks"][0]["task_id"], id);
    assert_eq!(second["syntax_tasks"]["new_blockers"], 0);
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.js"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
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
    let task =
        fs::read_to_string(fixture.0.join(".codeguard/tasks").join(format!("{id}.md"))).unwrap();
    assert!(task.contains("JavaScript"), "{task}");
    assert!(task.contains("结构"), "{task}");
    let reports = fixture.0.join(".codeguard/reports");
    let original: Value = fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter_map(|p| serde_json::from_slice::<Value>(&fs::read(p).unwrap()).ok())
        .find(|v| v["schema_version"] == "0.13.0")
        .unwrap();
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_WORKBENCH_REPORT_DIR") {
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
    fs::write(fixture.0.join("input.js"), "let x; { let x; }\n").unwrap();
    let clean = run(&fixture.0, &["check", "all"], None);
    assert_eq!(clean["syntax_tasks"]["tasks"], json!([]));
    let next = run(&fixture.0, &["next"], None);
    assert!(next.to_string().contains(id), "{next}");
}
