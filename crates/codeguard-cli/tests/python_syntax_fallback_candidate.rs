#![cfg(all(unix, feature = "wasm-precheck"))]

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-python-syntax-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn lint(&self, extra: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "python", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_configured_ruff_uses_bounded_python_wasm_without_native_approval() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::write(project.0.join("broken.py"), "def broken(\n").unwrap();
    let report = project.lint(&["--file", "broken.py"]);
    assert_eq!(report["schema_version"], "0.14.0");
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["files"][0]["run_status"], "incomplete");
    assert_eq!(report["files"][0]["reason"], "ruff_tool_not_found");
    assert!(
        report["files"][0]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report["syntax_precheck"]["backend"],
        "bundled_tree_sitter_wasm_candidate"
    );
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert_eq!(report["syntax_precheck"]["selected_files"], 1);
    assert_eq!(report["syntax_precheck"]["checked_files"], 1, "{report}");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(
        report["syntax_precheck"]["observations"][0]["path"],
        "broken.py"
    );
    assert_eq!(
        report["syntax_precheck"]["observations"][0]["classification"],
        "suspected"
    );
    assert_eq!(report["native"]["status"], "incomplete");
    assert_eq!(report["setup"]["requirement"], "required");
    assert!(report["setup"]["task_id"].is_null());
}

#[test]
fn valid_python_does_not_turn_unqualified_grammar_into_clean() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::write(
        project.0.join("good.py"),
        "def greet(name):\n    return name\n",
    )
    .unwrap();
    let report = project.lint(&["--file", "good.py"]);
    assert_eq!(report["schema_version"], "0.14.0");
    assert_eq!(report["syntax_precheck"]["checked_files"], 1, "{report}");
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn undeclared_ruff_still_provides_bounded_syntax_observation_without_claiming_ruff_missing() {
    let project = Project::new();
    fs::write(project.0.join("broken.py"), "def broken(\n").unwrap();
    let report = project.lint(&["--file", "broken.py"]);
    assert_eq!(report["schema_version"], "0.14.0");
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert_eq!(report["native"]["reason"], "project_ruff_config_not_found");
    assert_eq!(report["syntax_precheck"]["checked_files"], 1, "{report}");
    assert_eq!(
        report["syntax_precheck"]["observations"][0]["classification"],
        "suspected"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn invalid_ruff_config_does_not_get_laundered_by_wasm_precheck() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint\n").unwrap();
    fs::write(project.0.join("broken.py"), "def broken(\n").unwrap();
    let report = project.lint(&["--file", "broken.py"]);
    assert_eq!(report["schema_version"], "0.13.0");
    assert_eq!(report["files"][0]["reason"], "ruff_toml_invalid");
    assert!(report.get("syntax_precheck").is_none());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn initialized_workspace_keeps_one_python_native_confirmation_task_across_rescans() {
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    let source = root.join("broken.py");
    fs::write(&source, "def broken(\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let first = project.lint(&["--file", "broken.py"]);
    assert_eq!(first["schema_version"], "0.15.0");
    assert_eq!(first["task_persistence"]["status"], "synced_partial");
    let task_id = first["setup"]["task_id"].as_str().expect("真实任务 ID");
    let task_path = root.join(".codeguard/tasks").join(format!("{task_id}.md"));
    assert!(task_path.is_file());
    let fact_path = root
        .join(".codeguard/findings")
        .join(task_id)
        .join("finding.json");
    let fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    assert_eq!(fact["checker_id"], "python.ruff");
    assert_eq!(fact["reason_code"], "python_syntax_confirmation_needed");
    assert_eq!(fact["state"], "open");
    let task_text = fs::read_to_string(&task_path).unwrap();
    assert!(task_text.contains("Python 语法原生确认任务"));
    assert!(task_text.contains("原生复检"));
    let confirmation: Value = fs::read_dir(root.join(".codeguard/reports"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            serde_json::from_slice::<Value>(&fs::read(path).ok()?).ok()
        })
        .find(|report| report["report_type"] == "python_syntax_confirmation_observation")
        .unwrap();
    assert_eq!(confirmation["blocker_id"], task_id);
    assert_eq!(confirmation["scope"], "broken.py");
    assert_eq!(
        confirmation["observations"][0]["classification"],
        "suspected"
    );
    assert!(!confirmation.to_string().contains("def broken"));
    let second = project.lint(&["--file", "broken.py"]);
    assert_eq!(second["setup"]["task_id"], task_id);
    fs::write(&source, "def repaired():\n    return 1\n").unwrap();
    let changed = project.lint(&["--file", "broken.py"]);
    assert_eq!(changed["setup"]["task_id"], task_id);
    assert!(
        changed["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    assert_eq!(fact["state"], "open");
    assert_eq!(changed["delivery_decision"], "not_evaluated");
}

#[test]
fn forged_python_syntax_task_evidence_cannot_be_imported() {
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    fs::write(root.join("broken.py"), "def broken(\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let first = project.lint(&["--file", "broken.py"]);
    let task_id = first["setup"]["task_id"].as_str().unwrap();
    let report_dir = root.join(".codeguard/reports");
    let valid: Value = fs::read_dir(&report_dir)
        .unwrap()
        .filter_map(|entry| {
            serde_json::from_slice::<Value>(&fs::read(entry.ok()?.path()).ok()?).ok()
        })
        .find(|report| report["report_type"] == "python_syntax_confirmation_observation")
        .unwrap();
    let mut wrong_grammar = valid.clone();
    wrong_grammar["run_id"] = Value::String("python-syntax-999-1".into());
    wrong_grammar["grammar_sha256"] = Value::String("0".repeat(64));
    fs::write(
        report_dir.join("python-syntax-999-1.json"),
        serde_json::to_vec_pretty(&wrong_grammar).unwrap(),
    )
    .unwrap();
    let mut out_of_bounds = valid;
    out_of_bounds["run_id"] = Value::String("python-syntax-999-2".into());
    out_of_bounds["observations"][0]["start_line"] = Value::from(999);
    fs::write(
        report_dir.join("python-syntax-999-2.json"),
        serde_json::to_vec_pretty(&out_of_bounds).unwrap(),
    )
    .unwrap();
    let mut duplicate = wrong_grammar;
    duplicate["run_id"] = Value::String("python-syntax-999-3".into());
    duplicate["grammar_sha256"] = first["syntax_precheck"]["grammar_sha256"].clone();
    let encoded = serde_json::to_string(&duplicate).unwrap();
    let repeated = encoded.replacen(
        "\"grammar_sha256\":",
        "\"grammar_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"grammar_sha256\":",
        1,
    );
    fs::write(report_dir.join("python-syntax-999-3.json"), repeated).unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["work", "sync", root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(sync.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&sync.stdout).unwrap();
    assert_eq!(result["failed_reports"], 3, "{result}");
    assert_eq!(result["new_blockers"], 0);
    let fact: Value = serde_json::from_slice(
        &fs::read(
            root.join(".codeguard/findings")
                .join(task_id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn failed_task_persistence_is_visible_without_a_fabricated_task_id() {
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    fs::write(root.join("broken.py"), "def broken(\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let reports = root.join(".codeguard/reports");
    fs::remove_dir(&reports).unwrap();
    fs::write(&reports, b"occupied").unwrap();
    let report = project.lint(&["--file", "broken.py"]);
    assert_eq!(report["schema_version"], "0.15.0");
    assert_eq!(report["task_persistence"]["status"], "unavailable");
    assert_eq!(
        report["task_persistence"]["reason"],
        "reports_directory_unavailable"
    );
    assert!(report["setup"]["task_id"].is_null());
    assert_eq!(report["syntax_precheck"]["checked_files"], 1);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires native Ruff 0.16.8 via CODEGUARD_RUFF_BIN"]
fn native_ruff_task_verify_records_an_attempt_without_closing_candidate_task() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    fs::write(root.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::write(root.join("broken.py"), "def broken(\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let first = project.lint(&["--file", "broken.py"]);
    let id = first["setup"]["task_id"].as_str().unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            root.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        verify.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let report: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(report["task_id"], id);
    assert_eq!(report["native_scan"]["files"][0]["path"], "broken.py");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            root.join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}

#[test]
fn oversized_sibling_remains_visible_as_unavailable_without_erasing_other_suspicions() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::write(project.0.join("broken.py"), "def broken(\n").unwrap();
    fs::write(project.0.join("huge.py"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    let report = project.lint(&["--file", "broken.py", "--file", "huge.py"]);
    let precheck = &report["syntax_precheck"];
    assert_eq!(precheck["selected_files"], 2);
    assert_eq!(precheck["checked_files"], 1);
    assert_eq!(precheck["reason"], "partial_unavailable");
    assert_eq!(precheck["observations"][0]["path"], "broken.py");
    assert_eq!(precheck["unavailable"][0]["path"], "huge.py");
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
