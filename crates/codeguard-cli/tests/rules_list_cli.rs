use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cg-rules-list-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn list(&self, language: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "rules",
                "list",
                language,
                self.0.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn configured_checker_does_not_promote_candidate_rules_to_enabled_or_executed() {
    let project = Project::new();
    fs::write(project.0.join("a.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = []\n").unwrap();
    let (exit, report) = project.list("python");
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "rules_inventory");
    assert_eq!(report["authority"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let language = &report["languages"][0];
    assert_eq!(language["language"], "python");
    assert_eq!(
        language["checker_configurations"][0]["configuration"],
        "configured"
    );
    assert_eq!(language["rules"].as_array().unwrap().len(), 2);
    for rule in language["rules"].as_array().unwrap() {
        assert_eq!(rule["enabled"], "unverified");
        assert_eq!(rule["execution"], "not_run");
        assert_eq!(rule["approval"], "unverified");
        assert_eq!(rule["rulepack"]["sha256"].as_str().unwrap().len(), 64);
        assert!(
            rule["source_ref"]
                .as_str()
                .unwrap()
                .starts_with("https://docs.astral.sh/ruff/")
        );
    }
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn local_rulepack_and_approval_cannot_replace_bundled_mapping() {
    let project = Project::new();
    fs::write(project.0.join("a.py"), "pass\n").unwrap();
    fs::create_dir(project.0.join("rulepacks")).unwrap();
    let fake = br#"{"approved":true,"mappings":[{"native_rule_id":"LOCAL_FAKE"}]}"#;
    fs::write(project.0.join("rulepacks/ruff_lint_preview_v1.json"), fake).unwrap();
    fs::write(project.0.join("codeguard.json"), br#"{"approved":true}"#).unwrap();
    let (exit, report) = project.list("python");
    assert_eq!(exit, 3);
    assert!(!report.to_string().contains("LOCAL_FAKE"));
    assert_eq!(report["languages"][0]["rules"].as_array().unwrap().len(), 2);
    assert_eq!(
        fs::read(project.0.join("rulepacks/ruff_lint_preview_v1.json")).unwrap(),
        fake
    );
}

#[test]
fn all_preserves_missing_catalogs_and_does_not_execute_dynamic_config() {
    let project = Project::new();
    fs::write(project.0.join("a.ts"), "export const x = 1;\n").unwrap();
    fs::write(
        project.0.join("eslint.config.js"),
        "require('fs').writeFileSync('should_not_exist', 'executed'); module.exports=[];",
    )
    .unwrap();
    let (exit, report) = project.list("all");
    assert_eq!(exit, 3);
    let languages = report["languages"].as_array().unwrap();
    assert_eq!(languages.len(), 57);
    let java = languages
        .iter()
        .find(|row| row["language"] == "java")
        .unwrap();
    assert!(java["rules"].as_array().unwrap().is_empty());
    assert_eq!(java["catalog_gaps"].as_array().unwrap().len(), 6);
    let typescript = languages
        .iter()
        .find(|row| row["language"] == "typescript")
        .unwrap();
    assert!(
        typescript["checker_configurations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|checker| checker["checker_id"] == "node.eslint"
                && checker["configuration"] == "unknown")
    );
    assert!(!project.0.join("should_not_exist").exists());
}

#[test]
fn human_output_explains_candidates_and_help_exposes_read_only_entry() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["rules", "list", "python", project.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("F401"));
    assert!(text.contains("执行未运行"));
    assert!(text.contains("批准未核验"));
    let help = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("rules list <language|all>")
    );
}

#[test]
fn invalid_arguments_and_unknown_language_are_usage_errors() {
    let project = Project::new();
    for args in [
        vec!["rules", "list"],
        vec!["rules", "list", "not_a_language"],
        vec!["rules", "list", "python", "--format=sarif"],
        vec!["rules", "list", "python", "--format=json", "--format=human"],
        vec!["rules", "list", "python", "--apply"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .current_dir(&project.0)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn nonexistent_project_is_incomplete_not_empty_success() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "rules",
            "list",
            "java",
            project.0.join("missing").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["observation_complete"], false);
    assert_eq!(report["inspection_status"], "incomplete");
}
