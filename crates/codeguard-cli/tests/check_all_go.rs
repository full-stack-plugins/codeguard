#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(std::path::PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cg-check-all-go-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("go.mod"),
            "module example.com/all-go\n\ngo 1.23\n",
        )
        .unwrap();
        fs::write(path.join("main.go"), "package main\nfunc main() {}\n").unwrap();
        Self(path)
    }

    fn check(&self, tool: Option<&str>) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args(["check", "all"]).arg(&self.0);
        if let Some(tool) = tool {
            command.args(["--go-tool", tool]);
        }
        let output = command.args(["--format", "json"]).output().unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_all_keeps_go_tool_gap_visible() {
    let project = Project::new();
    let report = project.check(None);
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(
        report["native_results"]["go_lint"]["reason"],
        "go_tool_not_selected"
    );
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| { task["id"] == "go.lint" && task["status"] == "native_incomplete" })
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["language"] == "go"
                    && candidate["category"] == "lint"
                    && candidate["checker_id"] == "go.vet"
                    && candidate["status"] == "native_incomplete"
            })
    );
}

#[test]
#[ignore = "requires explicitly supplied local Go 1.23.4 binary"]
fn check_all_shows_real_go_finding_without_claiming_delivery() {
    let tool = std::env::var("CODEGUARD_GO_TOOL").unwrap();
    let project = Project::new();
    fs::write(
        project.0.join("main.go"),
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    let report = project.check(Some(&tool));
    assert_eq!(
        report["native_results"]["go_lint"]["findings"][0]["rule_id"],
        "printf"
    );
    assert_eq!(
        report["native_results"]["go_lint"]["coverage_proven"],
        false
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&project.0)
        .args(["--go-tool", &tool, "--format", "sarif"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("bad"));
}

#[test]
fn go_tool_parameter_is_scoped_and_requires_an_absolute_path() {
    for args in [
        vec!["check", "java", "--go-tool", "/usr/local/go/bin/go"],
        vec!["check", "all", "--go-tool", "relative/go"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
fn check_all_go_uses_shared_deadline_without_claiming_completion() {
    let project = Project::new();
    let tool = project.0.join("go-slow");
    fs::write(
        &tool,
        "#!/bin/sh\n/bin/sleep 3\nprintf '%s\\n' 'go version go1.23.4 darwin/arm64'\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&project.0)
        .arg("--go-tool")
        .arg(&tool)
        .args(["--timeout", "2s", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(
        report["native_results"]["go_lint"]["reason"],
        "request_deadline_exceeded"
    );
    assert_eq!(report["execution_tasks"][0]["status"], "deadline_exceeded");
}
