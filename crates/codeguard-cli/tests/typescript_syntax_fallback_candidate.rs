#![cfg(all(feature = "wasm-precheck", unix))]

use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(std::path::PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "codeguard-syntax-fallback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn lint(&self, name: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                self.0.join(name).to_str().unwrap(),
                "--format",
                "json",
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
fn missing_native_context_yields_scoped_suspected_observation_and_incomplete_gate() {
    let project = Project::new();
    fs::write(project.0.join("bad.ts"), "const x: number = ;\n").unwrap();
    let (status, report) = project.lint("bad.ts");
    assert_eq!(status, 3);
    assert_eq!(report["schema_version"], "0.3.0");
    assert_eq!(report["report_type"], "eslint_local_feedback");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["native"]["reason"], "explicit_context_missing");
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert!(
        !report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report["syntax_precheck"]["observations"][0]["classification"],
        "suspected"
    );
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn no_recovery_remains_incomplete_and_javascript_does_not_use_typescript_grammar() {
    let project = Project::new();
    fs::write(project.0.join("good.ts"), "const x: number = 1;\n").unwrap();
    let (_, report) = project.lint("good.ts");
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::write(project.0.join("app.js"), "const x = ;\n").unwrap();
    let (_, js_report) = project.lint("app.js");
    assert_eq!(js_report["schema_version"], "0.2.0");
    assert!(js_report.get("syntax_precheck").is_none());
}

#[test]
fn observed_project_local_eslint_candidate_is_not_treated_as_missing_native_lint() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const value: number = ;\n").unwrap();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(project.0.join("eslint.config.mjs"), "export default [];\n").unwrap();
    fs::create_dir_all(project.0.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        project.0.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("node_modules/eslint/bin/eslint.js"),
        "process.exit(99);\n",
    )
    .unwrap();

    let (status, report) = project.lint("app.ts");
    assert_eq!(status, 3);
    assert_eq!(report["reason"], "eslint_execution_context_missing");
    assert!(report.get("syntax_precheck").is_none());
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .contains("项目本地 ESLint 候选")
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn configured_but_uninstalled_eslint_still_allows_candidate_precheck() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const value: number = ;\n").unwrap();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(project.0.join("eslint.config.mjs"), "export default [];\n").unwrap();

    let (status, report) = project.lint("app.ts");
    assert_eq!(status, 3);
    assert_eq!(report["native"]["status"], "not_run");
    assert!(report.get("syntax_precheck").is_some());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn tsx_uses_its_own_grammar_instead_of_reporting_valid_jsx_as_suspect() {
    let project = Project::new();
    fs::write(
        project.0.join("component.tsx"),
        "const node = <div title=\"ok\">hello</div>;\n",
    )
    .unwrap();
    let (status, report) = project.lint("component.tsx");
    assert_eq!(status, 3);
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["syntax_precheck"]["language"], "tsx");
    assert_eq!(report["syntax_precheck"]["checked_files"], 1);
    assert_eq!(
        report["syntax_precheck"]["grammar_sha256"],
        "8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e"
    );
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");

    fs::write(
        project.0.join("broken.tsx"),
        "const node = <div title=></div>;\n",
    )
    .unwrap();
    let (_, broken) = project.lint("broken.tsx");
    assert_eq!(broken["syntax_precheck"]["language"], "tsx");
    assert!(
        !broken["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(broken["syntax_precheck"]["status"], "incomplete");
    assert!(broken["findings"].as_array().unwrap().is_empty());

    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/eslint-local-feedback-v0.4.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.4.0");
    assert_eq!(
        schema["properties"]["syntax_precheck"]["properties"]["language"]["const"],
        "tsx"
    );
    let actual: std::collections::BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let required: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(actual, required);
}

#[test]
fn candidate_feedback_schema_is_versioned_and_closed() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/eslint-local-feedback-v0.3.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.3.0");
    assert_eq!(schema["additionalProperties"], false);
    let project = Project::new();
    fs::write(project.0.join("good.ts"), "const x: number = 1;\n").unwrap();
    let (_, report) = project.lint("good.ts");
    let actual: std::collections::BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let required: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(actual, required);
}

#[test]
fn partial_native_context_or_symlink_does_not_trigger_candidate_worker() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const x: number = ;\n").unwrap();
    std::os::unix::fs::symlink(project.0.join("app.ts"), project.0.join("link.ts")).unwrap();
    let (_, linked) = project.lint("link.ts");
    assert_eq!(linked["schema_version"], "0.2.0");
    assert!(linked.get("syntax_precheck").is_none());

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("app.ts").to_str().unwrap(),
            "--config",
            project.0.join("eslint.config.js").to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let partial: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(partial["schema_version"], "0.2.0");
    assert!(partial.get("syntax_precheck").is_none());
}

#[test]
fn human_feedback_shows_suspected_location_and_native_confirmation_step() {
    let project = Project::new();
    fs::write(project.0.join("bad.ts"), "const x: number = ;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("bad.ts").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("内置语法初检"));
    assert!(text.contains("疑似语法"));
    assert!(text.contains("行 1，列"));
    assert!(text.contains("原生"));
}
