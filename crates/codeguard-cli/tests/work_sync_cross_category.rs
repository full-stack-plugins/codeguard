#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-sync-cross-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(root.join("src/main/java/A.java"), b"class A {}\n").unwrap();
        fs::write(root.join("app.py"), b"import os\n").unwrap();
        fs::write(root.join("ruff.toml"), b"[lint]\nselect = ['F']\n").unwrap();
        fs::write(root.join("pom.xml"), b"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build><dependencies><dependency><groupId>org.demo</groupId><artifactId>lib</artifactId><version>2.0</version></dependency></dependencies></project>").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        Self(root)
    }

    fn call(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn facts(&self, checker: &str) -> usize {
        fs::read_dir(self.0.join(".codeguard/findings"))
            .unwrap()
            .filter(|entry| {
                let fact: Value = serde_json::from_slice(
                    &fs::read(entry.as_ref().unwrap().path().join("finding.json")).unwrap(),
                )
                .unwrap();
                fact["checker_id"] == checker
            })
            .count()
    }

    fn event_count(&self) -> usize {
        fs::read_dir(self.0.join(".codeguard/findings"))
            .unwrap()
            .map(|entry| {
                fs::read_dir(entry.unwrap().path().join("events"))
                    .unwrap()
                    .count()
            })
            .sum()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn lint_then_cve_pending_reports_both_import_despite_a_bad_report() {
    let project = Project::new();
    let root = project.0.to_str().unwrap();
    let consumed = project.0.join(".codeguard/state/consumed");
    fs::remove_dir(&consumed).unwrap_or(());
    fs::write(&consumed, b"blocked").unwrap();

    let (_, lint) = project.call(&[
        "lint",
        "python",
        root,
        "--ruff-tool",
        "/nonexistent/codeguard-ruff",
        "--format=json",
    ]);
    assert_eq!(lint["backlog_status"], "backlog_update_failed");
    let lint_run = lint["run_id"].as_str().unwrap();
    let (_, check) = project.call(&["check", "java", root, "--format=json"]);
    let cve = &check["native_results"]["java_cve"];
    assert_eq!(cve["backlog_status"], "backlog_update_failed");
    let cve_run = cve["run_id"].as_str().unwrap();
    assert!(
        project
            .0
            .join(format!(".codeguard/reports/{lint_run}.json"))
            .is_file()
    );
    assert!(
        project
            .0
            .join(format!(".codeguard/reports/{cve_run}.json"))
            .is_file()
    );
    assert_eq!(project.facts("python.ruff"), 0);
    assert_eq!(project.facts("java.maven.dependency_check"), 0);

    fs::remove_file(&consumed).unwrap();
    fs::create_dir(&consumed).unwrap();
    fs::write(project.0.join(".codeguard/reports/bad.json"), b"not-json").unwrap();
    let (_, sync) = project.call(&["work", "sync", root, "--format=json"]);
    assert!(sync["imported_reports"].as_u64().unwrap() >= 2, "{sync}");
    assert_eq!(sync["failed_reports"], 1);
    assert_eq!(project.facts("python.ruff"), 1);
    assert_eq!(project.facts("java.maven.dependency_check"), 1);
    assert!(consumed.join(format!("{lint_run}.json")).is_file());
    assert!(consumed.join(format!("{cve_run}.json")).is_file());
    let (_, next) = project.call(&["next", root, "--format=json"]);
    assert_eq!(next["reason"], "failed_report_requires_repair");
    assert_eq!(next["failed_reports"][0]["run_id"], "bad");
    assert_eq!(next["failed_reports"][0]["reason"], "report_invalid_json");
    assert_eq!(next["delivery_decision"], "not_evaluated");
    let (_, status) = project.call(&["status", root, "--format=json"]);
    assert_eq!(status["pending_reports"], true);
    assert_eq!(status["next"]["reason"], "failed_report_requires_repair");

    let bad = project.0.join(".codeguard/reports/bad.json");
    fs::write(&bad, b"still-not-json").unwrap();
    let (_, changed) = project.call(&["next", root, "--format=json"]);
    assert_eq!(changed["reason"], "pending_reports_require_sync");
    let (_, resynced) = project.call(&["work", "sync", root, "--format=json"]);
    assert_eq!(resynced["failed_reports"], 1);
    let (_, current) = project.call(&["next", root, "--format=json"]);
    assert_eq!(current["reason"], "failed_report_requires_repair");

    let (_, repeated) = project.call(&["work", "sync", root, "--format=json"]);
    assert_eq!(repeated["new_blockers"], 0);
    assert_eq!(repeated["failed_reports"], 1);
    assert_eq!(project.facts("python.ruff"), 1);
    assert_eq!(project.facts("java.maven.dependency_check"), 1);
    let events_before_retry = project.event_count();
    fs::remove_file(consumed.join(format!("{lint_run}.json"))).unwrap();
    fs::remove_file(consumed.join(format!("{cve_run}.json"))).unwrap();
    let (_, recovered) = project.call(&["work", "sync", root, "--format=json"]);
    assert!(recovered["imported_reports"].as_u64().unwrap() >= 2);
    assert_eq!(recovered["failed_reports"], 1);
    assert_eq!(recovered["new_blockers"], 0);
    assert_eq!(project.event_count(), events_before_retry);
    assert!(consumed.join(format!("{lint_run}.json")).is_file());
    assert!(consumed.join(format!("{cve_run}.json")).is_file());
}
