#![cfg(unix)]
//! Python 独立注释入口；受控运输不证明原生语义或生产资格。
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
impl Project {
    fn new(config: bool) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-py-comments-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("app.py"),
            "import os\ndef f() -> int:\n    return 1\n",
        )
        .unwrap();
        if config {
            fs::write(
                root.join("ruff.toml"),
                "preview=true\n[lint]\nselect=['DOC201','F401']\n",
            )
            .unwrap();
        }
        let p = Self(root);
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", "--apply", "--format=json"])
            .arg(&p.0)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        p
    }
    fn tool(&self) -> PathBuf {
        let path = self.0.join("ruff");
        fs::write(&path, r#"#!/bin/sh
printf '%s\n' "$1" >> calls
if [ "$1" = '--version' ]; then echo 'ruff 0.16.8'; exit 0; fi
if [ "$2" = '--show-files' ]; then echo "$3"; exit 0; fi
if [ "$2" = '--show-settings' ]; then printf 'linter.rules.enabled = [\n\tmissing-return (DOC201),\n\tunused-import (F401),\n]\nlinter.per_file_ignores = {}\n'; exit 0; fi
if [ "$2" = '--no-cache' ]; then
 if [ "$3" = '--ignore-noqa' ]; then source=$6; else source=$5; fi
 if [ -f fixed ]; then printf '[]\n'; exit 0; fi
 printf '[{"code":"DOC201","message":"private doc text","filename":"%s","location":{"row":2,"column":1},"severity":"error"},{"code":"F401","message":"private import text","filename":"%s","location":{"row":1,"column":1},"severity":"error"}]\n' "$source" "$source"; exit 1
fi
exit 2
"#).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn comments(&self, extra: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "python", "--format=json"])
            .arg(&self.0)
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn doc_candidates_reuse_original_tasks_and_do_not_choose_convention_finding() {
    let p = Project::new(true);
    let tool = p.tool();
    let first = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(first["report_type"], "python_comments_feedback");
    assert_eq!(first["documentation_findings"].as_array().unwrap().len(), 1);
    assert_eq!(first["documentation_findings"][0]["rule_id"], "DOC201");
    assert_eq!(
        first["native_report"]["files"][0]["findings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let id = first["documentation_findings"][0]["finding_id"].clone();
    assert_eq!(first["next"]["repair_brief"]["task_id"], id);
    let repeat = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(repeat["documentation_findings"][0]["finding_id"], id);
    assert_eq!(repeat["detailed_contract_qualification"], "not_granted");
    assert!(!first.to_string().contains("private doc text"));
    assert!(!first.to_string().contains("private import text"));
    fs::write(p.0.join("fixed"), "").unwrap();
    let clean = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(clean["documentation_findings"], json!([]));
    assert_eq!(clean["next"]["repair_brief"]["task_id"], id);
}

#[test]
fn missing_configuration_is_a_preparation_task_and_never_executes_selected_tool() {
    let p = Project::new(false);
    let tool = p.tool();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(
        report["documentation_configuration"]["status"],
        "unavailable"
    );
    assert_eq!(
        report["documentation_configuration"]["files"][0]["globally_enabled_documentation_rules"],
        Value::Null
    );
    assert_eq!(report["documentation_findings"], json!([]));
    assert_eq!(report["next"]["repair_brief"]["kind"], "blocker");
    assert!(!p.0.join("calls").exists());
    assert!(!p.0.join("ruff.toml").exists());
    save_configuration_case("missing-configuration", &report);
}

#[test]
fn uninitialized_comments_keeps_native_evidence_without_creating_workspace() {
    let p = Project::new(true);
    fs::remove_dir_all(p.0.join(".codeguard")).unwrap();
    let tool = p.tool();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(
        report["native_report"]["workspace_binding"],
        "uninitialized"
    );
    assert_eq!(
        report["documentation_findings"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["next"], Value::Null);
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn unknown_doc_rule_prefix_does_not_create_a_documentation_candidate() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("DOC201", "DOC999"),
    )
    .unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["documentation_findings"], json!([]));
    assert_ne!(report["next"]["repair_brief"]["native_rule_id"], "DOC999");
}

#[test]
fn malformed_arguments_stop_before_native_execution() {
    let p = Project::new(true);
    let tool = p.tool();
    for extra in [
        vec!["--format=json", "--format=human"],
        vec!["--timeout", "0s"],
        vec!["--ruff-tool", "relative"],
        vec!["--file", "../app.py"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "python"])
            .arg(&p.0)
            .args(["--ruff-tool", tool.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!p.0.join("calls").exists());
    }
}

#[test]
fn native_timeout_stays_incomplete_without_a_documentation_clean_claim() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("echo 'ruff 0.16.8'", "/bin/sleep 1; echo 'ruff 0.16.8'"),
    )
    .unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap(), "--timeout", "100ms"]);
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["local_scan_complete"], false);
    save_configuration_case("timeout", &report);
    assert_eq!(report["documentation_rule_coverage"], "unverified");
    assert!(
        report["native_report"]["incomplete_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "request_deadline_exceeded"),
        "{report}"
    );
}

#[test]
fn nonexistent_root_keeps_structured_unavailable_native_report() {
    let p = Project::new(true);
    let tool = p.tool();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "python",
            "--format=json",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ])
        .arg(p.0.join("missing"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native_report"], Value::Null);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["reason"], "project_root_unavailable");
    save_configuration_case("root-unavailable", &report);
    assert!(!p.0.join("calls").exists());
}

#[test]
fn interrupted_native_probe_returns_cancelled_without_a_clean_claim() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("echo 'ruff 0.16.8'", "/bin/sleep 10; echo 'ruff 0.16.8'"),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "python",
            "--format=json",
            "--timeout",
            "5s",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ])
        .arg(&p.0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let wait_until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !p.0.join("calls").exists() && std::time::Instant::now() < wait_until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !p.0.join("calls").exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("原生版本探针未启动");
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    save_configuration_case("cancelled", &report);
    assert_eq!(report["native_report"]["command_status"], "cancelled");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["documentation_rule_coverage"], "unverified");
}

#[test]
fn clean_selected_documentation_rules_are_observed_without_repeating_native_calls() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(p.0.join("fixed"), "").unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["schema_version"], "0.2.0");
    save_configuration_case("clean-selected", &report);
    let configuration = &report["documentation_configuration"];
    assert_eq!(configuration["status"], "observed");
    assert_eq!(configuration["selected_file_count"], 1);
    assert_eq!(
        configuration["files"][0]["globally_enabled_documentation_rules"],
        json!(["DOC201"])
    );
    assert_eq!(report["documentation_findings"], json!([]));
    assert_eq!(report["detailed_contract_qualification"], "not_granted");
    assert_eq!(
        report["native_report"]["files"][0]["rule_settings"]["globally_enabled_mapped_rules"],
        json!(["F401"])
    );
    let calls = fs::read_to_string(p.0.join("calls")).unwrap();
    fs::remove_file(p.0.join("calls")).unwrap();
    let original = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            "--format=json",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ])
        .arg(&p.0)
        .output()
        .unwrap();
    assert_eq!(original.status.code(), Some(3));
    assert_eq!(fs::read_to_string(p.0.join("calls")).unwrap(), calls);
    let native_file = &report["native_report"]["files"][0];
    let observed_file = &configuration["files"][0];
    for key in ["config_sha256", "source_sha256", "tool_sha256"] {
        assert_eq!(observed_file[key], native_file[key]);
    }
    assert_eq!(
        observed_file["settings_sha256"],
        native_file["rule_settings"]["settings_sha256"]
    );
}

#[test]
fn clean_unselected_documentation_rules_require_configuration_without_modifying_it() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(p.0.join("fixed"), "").unwrap();
    fs::write(
        &tool,
        fs::read_to_string(&tool)
            .unwrap()
            .replace("\\tmissing-return (DOC201),\\n", ""),
    )
    .unwrap();
    let before = fs::read(p.0.join("ruff.toml")).unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["documentation_configuration"]["status"], "observed");
    assert_eq!(
        report["documentation_configuration"]["unselected_file_count"],
        1
    );
    assert_eq!(
        report["documentation_configuration"]["files"][0]["status"],
        "not_selected"
    );
    assert_eq!(
        report["documentation_configuration"]["files"][0]["globally_enabled_documentation_rules"],
        json!([])
    );
    assert!(
        report["documentation_configuration"]["next_action"]
            .as_str()
            .unwrap()
            .contains("文档规则")
    );
    assert_eq!(fs::read(p.0.join("ruff.toml")).unwrap(), before);
    save_configuration_case("unselected", &report);
}

#[test]
fn per_file_ignores_remain_unresolved_despite_global_documentation_selection() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::write(p.0.join("fixed"), "").unwrap();
    fs::write(
        &tool,
        fs::read_to_string(&tool).unwrap().replace(
            "linter.per_file_ignores = {}\\n",
            "linter.per_file_ignores = {\\n    app.py: DOC201\\n}\\n",
        ),
    )
    .unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    save_configuration_case("per-file-ignore", &report);
    assert_eq!(
        report["documentation_configuration"]["files"][0]["per_file_ignores_present"],
        true
    );
    assert_eq!(
        report["documentation_configuration"]["coverage_proven"],
        false
    );
    assert_eq!(
        report["documentation_configuration"]["scope"],
        "global_rule_selection_only"
    );
}

#[test]
fn nested_configuration_missing_doc_rules_stays_visible_in_per_file_observation() {
    let p = Project::new(true);
    fs::create_dir(p.0.join("nested")).unwrap();
    fs::write(p.0.join("nested/app.py"), "value = 1\n").unwrap();
    fs::write(p.0.join("nested/ruff.toml"), "[lint]\nselect=['F401']\n").unwrap();
    let tool = p.tool();
    fs::write(p.0.join("fixed"), "").unwrap();
    fs::write(&tool, fs::read_to_string(&tool).unwrap().replace("if [ \"$2\" = '--show-settings' ]; then", "if [ \"$2\" = '--show-settings' ]; then case \"$3\" in */nested/*) printf 'linter.rules.enabled = [\\n\\tunused-import (F401),\\n]\\nlinter.per_file_ignores = {}\\n'; exit 0;; esac;")).unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    let partial = &report["documentation_configuration"];
    assert_eq!(partial["status"], "partial");
    save_configuration_case("partial", &report);
    assert_eq!(partial["unavailable_file_count"], 1);
    assert_eq!(
        partial["files"][1]["globally_enabled_documentation_rules"],
        Value::Null
    );
    // 子根运输尚报未启用规则时保留未知；修正子根运输后才能观察未选择文档规则。
    fs::write(p.0.join("nested/fixed"), "").unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    let observed = &report["documentation_configuration"];
    assert_eq!(observed["status"], "observed", "{report}");
    assert_eq!(observed["selected_file_count"], 1);
    assert_eq!(observed["unselected_file_count"], 1);
    let nested = observed["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "nested/app.py")
        .unwrap();
    assert_eq!(nested["config_ref"], "nested/ruff.toml");
    assert_eq!(nested["status"], "not_selected");
    assert_eq!(observed["coverage_proven"], false);
    save_configuration_case("nested-observed", &report);
}

fn save_configuration_case(name: &str, report: &Value) {
    if let Ok(dir) = std::env::var("CODEGUARD_TEST_PYTHON_DOC_CONFIG_CASE_EVIDENCE") {
        let dir = PathBuf::from(dir);
        assert!(dir.is_absolute());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn absence_of_python_sources_does_not_become_observed_documentation_configuration() {
    let p = Project::new(true);
    let tool = p.tool();
    fs::remove_file(p.0.join("app.py")).unwrap();
    let report = p.comments(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(
        report["documentation_configuration"]["status"],
        "no_python_sources"
    );
    assert_eq!(
        report["documentation_configuration"]["source_file_count"],
        0
    );
    assert_eq!(
        report["documentation_configuration"]["coverage_proven"],
        false
    );
    save_configuration_case("no-python-sources", &report);
}
