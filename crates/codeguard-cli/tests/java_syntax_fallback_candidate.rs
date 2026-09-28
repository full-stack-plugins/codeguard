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
            "codeguard-java-syntax-fallback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn lint(&self, file: &str, extra: &[&str]) -> (i32, Value) {
        let mut args = vec!["lint", "java", file, "--format=json"];
        args.extend(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
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
fn missing_native_context_reports_suspected_java_syntax_without_confirmed_findings() {
    let project = Project::new();
    let path = project.0.join("Broken.java");
    fs::write(&path, "class Broken {\n").unwrap();
    let (status, report) = project.lint(path.to_str().unwrap(), &[]);
    assert_eq!(status, 3);
    assert_eq!(report["report_type"], "java_syntax_precheck_feedback");
    assert_eq!(report["schema_version"], "0.1.0");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["syntax_precheck"]["language"], "java");
    assert_eq!(report["syntax_precheck"]["source_path"], "Broken.java");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
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
    assert_eq!(report["setup"]["requirement"], "required");
    assert!(report["setup"]["task_id"].is_null());
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn no_recovery_is_still_incomplete_and_not_a_native_pass() {
    let project = Project::new();
    let path = project.0.join("Good.java");
    fs::write(&path, "class Good {}\n").unwrap();
    let (status, report) = project.lint(path.to_str().unwrap(), &[]);
    assert_eq!(status, 3);
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["coverage_proven"], false);
}

#[test]
fn oversized_source_keeps_worker_incomplete_without_source_violation() {
    let project = Project::new();
    let path = project.0.join("Large.java");
    fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let (status, report) = project.lint(path.to_str().unwrap(), &[]);
    assert_eq!(status, 3);
    assert_eq!(report["syntax_precheck"]["reason"], "source_unavailable");
    assert_eq!(report["syntax_precheck"]["checked_files"], 0);
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["native"]["status"], "not_run");
}

#[test]
fn partial_native_context_javadoc_symlink_and_other_extension_keep_native_path() {
    let project = Project::new();
    let path = project.0.join("Broken.java");
    fs::write(&path, "class Broken {\n").unwrap();
    let link = project.0.join("Link.java");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    fs::write(project.0.join("Broken.txt"), "class Broken {\n").unwrap();
    for (file, extra) in [
        (path.as_path(), vec!["--java-home", "/missing/jdk"]),
        (path.as_path(), vec!["--checker", "javadoc"]),
        (link.as_path(), vec![]),
    ] {
        let (_, report) = project.lint(file.to_str().unwrap(), &extra);
        assert!(report.get("syntax_precheck").is_none());
    }
    let (_, other) = project.lint(project.0.join("Broken.txt").to_str().unwrap(), &[]);
    assert!(other.get("syntax_precheck").is_none());
}

#[test]
fn human_output_describes_suspicion_and_native_confirmation() {
    let project = Project::new();
    let path = project.0.join("Broken.java");
    fs::write(&path, "class Broken {\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "java", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("疑似"));
    assert!(text.contains("原生"));
    assert!(text.contains("行 1"));
}

#[test]
fn candidate_feedback_schema_is_versioned_and_closed() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/java-syntax-precheck-feedback-v0.1.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
    assert_eq!(schema["additionalProperties"], false);
    let project = Project::new();
    let path = project.0.join("Good.java");
    fs::write(&path, "class Good {}\n").unwrap();
    let (_, report) = project.lint(path.to_str().unwrap(), &[]);
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
