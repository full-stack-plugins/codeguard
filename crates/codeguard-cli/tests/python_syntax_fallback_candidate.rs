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
