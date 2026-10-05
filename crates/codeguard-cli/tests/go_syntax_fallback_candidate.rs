#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-go-fallback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("go.mod"),
            "module example.test/sample\n\ngo 1.23\n",
        )
        .unwrap();
        fs::write(root.join("main.go"), source).unwrap();
        Self(root)
    }
    fn command(&self, args: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg(&self.0)
            .args(["--format", "json"])
            .env("PATH", "")
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
fn missing_native_go_reports_structure_and_requires_native_confirmation() {
    let project = Project::new("// package comment\nfunc f() {}\n");
    let report = project.command(&["lint", "go"]);
    assert_eq!(
        report["report_type"], "go_lint_fallback_feedback",
        "{report}"
    );
    assert_eq!(
        report["syntax_candidates"]["observations"][0]["structural_observation_count"],
        1
    );
    assert_eq!(report["native_tool_requirement"], "required");
    assert_eq!(report["syntax_tasks"]["status"], "incomplete");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        fs::read_to_string(project.0.join("main.go")).unwrap(),
        "// package comment\nfunc f() {}\n"
    );
}
#[test]
fn zero_candidates_recommend_native_tool_without_claiming_lint_passed() {
    let project = Project::new("package main\nfunc f() {}\n");
    let report = project.command(&["lint", "go"]);
    assert_eq!(report["native_tool_requirement"], "recommended", "{report}");
    assert_eq!(report["preliminary_result"], "no_candidates_observed");
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["native_report"]["native_status"], "incomplete");
}
#[test]
fn fallback_and_aggregate_reuse_the_same_task_after_initialization() {
    let project = Project::new("func f() {}\n");
    project.command(&["init", "--apply"]);
    let first = project.command(&["lint", "go"]);
    let id = first["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    let second = project.command(&["lint", "go"]);
    assert_eq!(second["syntax_tasks"]["tasks"][0]["task_id"], id);
    let aggregate = project.command(&["check", "go"]);
    assert!(
        aggregate["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["task_id"] == id)
    );
    fs::write(project.0.join("main.go"), "package main\nfunc f() {}\n").unwrap();
    let fixed = project.command(&["lint", "go"]);
    assert_eq!(fixed["preliminary_result"], "no_candidates_observed");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
}
#[test]
fn explicitly_invalid_native_tool_is_not_silently_replaced_by_wasm() {
    let project = Project::new("func f() {}\n");
    let report = project.command(&["lint", "go", "--go-tool", "/missing/sdk/go"]);
    assert_eq!(report["report_type"], "go_lint_local_feedback");
    assert_eq!(report["reason"], "go_tool_unavailable");
    assert!(report.get("syntax_candidates").is_none());
}

#[test]
fn unreadable_go_input_keeps_preliminary_result_incomplete() {
    let project = Project::new("package main\nfunc f() {}\n");
    fs::write(project.0.join("bad.go"), [0xff, 0xfe]).unwrap();
    let report = project.command(&["lint", "go"]);
    assert_eq!(report["preliminary_result"], "incomplete", "{report}");
    assert_eq!(report["native_tool_requirement"], "required");
}

#[test]
fn absolute_path_native_selection_preempts_fallback_even_on_version_failure() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new("func f() {}\n");
    let tool = project.0.join("go");
    fs::write(&tool, "#!/bin/sh\nprintf 'not a supported Go version\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&project.0)
        .args(["--format", "json"])
        .env("PATH", &project.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["report_type"], "go_lint_local_feedback");
    assert_eq!(report["reason"], "go_tool_version_unvalidated");
    assert!(report.get("syntax_candidates").is_none());
}

#[test]
fn relative_path_tool_is_ignored_and_equal_format_parameter_is_supported() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new("package main\nfunc f() {}\n");
    fs::write(
        project.0.join("go"),
        "#!/bin/sh\nprintf 'unsupported go version\n'\n",
    )
    .unwrap();
    fs::set_permissions(project.0.join("go"), fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&project.0)
        .arg("--format=json")
        .current_dir(&project.0)
        .env("PATH", ".")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["tool_selection"]["source"], "not_found");
    assert_eq!(report["native_tool_requirement"], "recommended");
}
