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
        let root = std::env::temp_dir().join(format!("codeguard-plan-{}-{id}", std::process::id()));
        fs::create_dir(&root).expect("fixture root");
        Self(root)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture");
    }
}

#[test]
fn plan_reports_configured_candidates_without_certifying_obligations_or_running_tools() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").expect("source");
    fs::write(
        project.0.join("pyproject.toml"),
        "[tool.ruff.lint]\nselect = ['F']\n",
    )
    .expect("config");
    let marker = project.0.join("native-tool-ran");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).expect("fake tool");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).expect("executable");
    }
    let before: Vec<_> = fs::read_dir(&project.0)
        .expect("list before")
        .map(|item| item.expect("entry").file_name())
        .collect();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "plan",
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .env("PATH", &project.0)
        .output()
        .expect("run plan");
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON preview");
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/plan-preview.schema.json"))
            .expect("published preview schema");
    let expected: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    let actual: std::collections::BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(report["report_type"], "plan_preview");
    assert_eq!(report["planning_status"], "incomplete");
    assert_eq!(report["quality_decision"], "not_evaluated");
    assert!(report["policy_identity"].is_null());
    assert_eq!(report["obligations"], serde_json::json!([]));
    assert_eq!(report["selected_categories"].as_array().unwrap().len(), 6);
    assert!(
        report["observed_checkers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["checker_id"] == "python.ruff" && item["configuration"] == "configured"
            })
    );
    assert!(
        report["candidate_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["checker_id"] == "python.ruff" && item["command"].is_null() })
    );
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item == "trusted_policy_unavailable" })
    );
    assert!(!marker.exists(), "plan cannot execute native tool");
    let after: Vec<_> = fs::read_dir(&project.0)
        .expect("list after")
        .map(|item| item.expect("entry").file_name())
        .collect();
    assert_eq!(before, after, "plan cannot write project state");
}

#[test]
fn invalid_selection_fails_before_project_observation() {
    let project = Project::new();
    let missing = project.0.join("missing");
    for selection in [["style", "python"], ["lint", "unknown-language"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "plan",
                selection[0],
                selection[1],
                missing.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .expect("run invalid plan");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn java_lint_plan_sees_p3c_declaration_but_does_not_claim_rule_execution() {
    let project = Project::new();
    fs::write(
        project.0.join("pom.xml"),
        include_str!("../../../tests/fixtures/p3c_native/pom.xml"),
    )
    .expect("P3C POM");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "plan",
            "lint",
            "java",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .expect("run Java plan");
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).expect("preview");
    assert_eq!(report["selected_categories"], serde_json::json!(["lint"]));
    assert!(
        report["candidate_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["checker_id"] == "java.maven.p3c" && item["command"].is_null() })
    );
    assert!(report["obligations"].as_array().unwrap().is_empty());
    assert_eq!(report["quality_decision"], "not_evaluated");
}
