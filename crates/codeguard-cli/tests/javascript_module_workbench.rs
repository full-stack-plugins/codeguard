#![cfg(all(feature = "wasm-precheck", unix))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(root: &Path, args: &[&str], input: Option<Value>) -> Value {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    if args[0] == "lint" && args[1] == "typescript" {
        cmd.args(args)
            .arg(root.join("input.mjs"))
            .arg("--workspace")
            .arg(root);
    } else if args[0] == "hook" {
        cmd.args(&args[..2]).arg(root).args(&args[2..]);
    } else {
        cmd.args(args).arg(root);
    }
    let mut child = cmd
        .arg("--format=json")
        .env("PATH", "/no/tools")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(v) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(v.to_string().as_bytes())
            .unwrap();
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().unwrap();
    assert!(matches!(output.status.code(), Some(0 | 3)), "{output:?}");
    let value = serde_json::from_slice(&output.stdout).unwrap();
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODULE_WORKBENCH_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join(format!(
                "{}-{}.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            &output.stdout,
        )
        .unwrap();
    }
    value
}
#[test]
fn module_fallback_reuses_task_across_project_lint_and_edit_without_commonjs_false_positives() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-js-module-workbench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("package.json"), b"{\"type\":\"module\"}").unwrap();
    for path in ["input.mjs", "input.cjs", "input.js"] {
        fs::write(fixture.0.join(path), b"return 1;\n").unwrap();
    }
    run(&fixture.0, &["init", "--apply"], None);
    let first = run(&fixture.0, &["check", "all"], None);
    let rows = first["syntax_candidates"]["observations"]
        .as_array()
        .unwrap();
    let mjs = rows.iter().find(|r| r["path"] == "input.mjs").unwrap();
    assert_eq!(mjs["structural_observation_count"], 1, "{first}");
    assert_eq!(mjs["javascript_mode_observation"]["mode"], "module");
    assert_eq!(
        mjs["structural_observations"][0]["rule_id"],
        "codeguard.javascript.module_return_outside_function"
    );
    let js = rows.iter().find(|r| r["path"] == "input.js").unwrap();
    assert_eq!(js["structural_observation_count"], 1);
    let cjs = rows.iter().find(|r| r["path"] == "input.cjs").unwrap();
    assert!(cjs.get("structural_observations").is_none());
    assert!(cjs.get("javascript_mode_observation").is_none());
    assert_eq!(first["schema_version"], "0.60.0");
    assert_eq!(first["syntax_tasks"]["failures"], json!([]));
    let id = first["syntax_tasks"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["path"] == "input.mjs")
        .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = run(&fixture.0, &["check", "all"], None);
    assert_eq!(second["syntax_tasks"]["new_blockers"], 0);
    assert!(
        second["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["task_id"] == id)
    );
    let lint = run(&fixture.0, &["lint", "typescript"], None);
    assert_eq!(lint["schema_version"], "0.7.0");
    assert_eq!(lint["setup"]["task_id"], id, "{lint}");
    assert_eq!(lint["setup"]["requirement"], "required");
    let lint_all = run(&fixture.0, &["lint", "all"], None);
    assert_eq!(lint_all["schema_version"], "0.60.0");
    assert_eq!(lint_all["requested_categories"], json!(["lint"]));
    assert!(
        lint_all["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["task_id"] == id)
    );
    let event = json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["input.mjs"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    let hook = run(
        &fixture.0,
        &["hook", "execute", "--timeout", "30s"],
        Some(event),
    );
    assert_eq!(
        hook["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        id
    );
    let next = run(&fixture.0, &["next"], None);
    assert!(next.to_string().contains("module"), "{next}");
    let reports = fixture.0.join(".codeguard/reports");
    let original: Value = fs::read_dir(&reports)
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .find(|v| v["schema_version"] == "0.15.0" && v["scope"] == "input.js")
        .unwrap();
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODULE_WORKBENCH_REPORT_DIR") {
        fs::write(
            PathBuf::from(dir).join("syntax-confirmation-valid.json"),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
    }
    for index in 0..5 {
        let mut forged = original.clone();
        forged["run_id"] = json!(format!("syntax-confirm-999-{}", 2000 + index));
        match index {
            0 => {
                forged["observations"][0]["javascript_mode_observation"]["package_sha256"] =
                    json!("0".repeat(64))
            }
            1 => {
                forged["observations"][0]["javascript_mode_observation"]["mode"] = json!("commonjs")
            }
            2 => {
                forged["observations"][0]["javascript_mode_observation"]["searched_directories"] =
                    json!(["other"])
            }
            3 => {
                forged["observations"][0]["structural_observations"][0]["rule_sha256"] =
                    json!("0".repeat(64))
            }
            _ => {
                forged["schema_version"] = json!("0.13.0");
                forged["observations"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("javascript_mode_observation");
            }
        }
        fs::write(
            reports.join(format!("{}.json", forged["run_id"].as_str().unwrap())),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
    }
    let sync = run(&fixture.0, &["work", "sync"], None);
    assert_eq!(sync["failed_reports"], 5, "{sync}");
    assert_eq!(sync["new_findings"], 0);
    assert_eq!(sync["new_blockers"], 0);
    for index in 0..5 {
        fs::remove_file(reports.join(format!("syntax-confirm-999-{}.json", 2000 + index))).unwrap();
    }
    fs::write(fixture.0.join("input.mjs"), b"const x=1;\n").unwrap();
    let clean = run(&fixture.0, &["check", "all"], None);
    assert!(
        !clean["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["task_id"] == id)
    );
    let next = run(&fixture.0, &["next"], None);
    assert!(next.to_string().contains("module"), "{next}");
    let task =
        fs::read_to_string(fixture.0.join(".codeguard/tasks").join(format!("{id}.md"))).unwrap();
    assert!(!task.contains("[x]"), "{task}");
    let js_id = first["syntax_tasks"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["path"] == "input.js")
        .unwrap()["task_id"]
        .as_str()
        .unwrap();
    fs::write(fixture.0.join("package.json"), b"{\"type\":\"commonjs\"}").unwrap();
    let node = fixture.0.join("never_node");
    let marker = fixture.0.join("node_executed");
    fs::write(
        &node,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    let entry = fixture.0.join("eslint.js");
    let config = fixture.0.join("eslint.config.js");
    fs::write(&entry, b"test").unwrap();
    fs::write(&config, b"test").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["task", "verify", js_id])
        .arg(&fixture.0)
        .arg("--node-tool")
        .arg(&node)
        .arg("--eslint-entry")
        .arg(&entry)
        .args(["--eslint-version", "10.0.0", "--config"])
        .arg(&config)
        .arg("--cwd")
        .arg(&fixture.0)
        .arg("--format=json")
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let verified: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        verified["native_scan"]["reason"], "javascript_module_context_changed",
        "{verified}"
    );
    assert!(!marker.exists());
}
