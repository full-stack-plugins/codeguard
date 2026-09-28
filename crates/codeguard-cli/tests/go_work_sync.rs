use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-go-sync-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("go.mod"), "module example.com/tasks\n\ngo 1.23\n").unwrap();
        fs::write(
            root.join("main.go"),
            "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&root)
            .args(["--apply", "--format", "json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        Self(root)
    }
    fn lint(&self, tool: Option<&str>) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(["lint", "go"])
            .arg(&self.0)
            .args(["--format", "json"]);
        if let Some(tool) = tool {
            c.args(["--go-tool", tool]);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn query(&self, args: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg(&self.0)
            .args(["--format", "json"])
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn missing_go_tool_creates_one_stable_environment_task_and_next() {
    let p = Project::new();
    let first = p.lint(None);
    assert_eq!(first["backlog_status"], "synced_partial");
    assert_eq!(first["backlog_sync"]["new_blockers"], 1);
    let again = p.lint(None);
    assert_eq!(again["backlog_sync"]["new_blockers"], 0);
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    let next = p.query(&["next"]);
    assert_eq!(next["repair_brief"]["checker_id"], "go.vet", "{next}");
    assert!(next.to_string().contains("--go-tool"));
    assert!(!next.to_string().contains("ruff"));
    let sync = p.query(&["work", "sync"]);
    assert_eq!(sync["imported_reports"], 0);
    assert_eq!(sync["failed_reports"], 0);
    let check = p.query(&["check", "all"]);
    assert_eq!(
        check["native_results"]["go_lint"]["backlog_status"], "synced_partial",
        "{check}"
    );
    assert_eq!(
        check["native_results"]["go_lint"]["backlog_sync"]["new_blockers"],
        0
    );
}
#[test]
#[ignore = "requires explicit local Go 1.23.4; no install or network"]
fn real_go_findings_sync_without_duplicate_tasks_or_false_closure() {
    let tool = std::env::var("CODEGUARD_GO_TOOL").unwrap();
    let p = Project::new();
    let first = p.lint(Some(&tool));
    assert_eq!(first["backlog_status"], "synced_partial", "{first}");
    assert_eq!(first["backlog_sync"]["new_findings"], 1);
    let id = first["findings"][0]["finding_id"].as_str().unwrap();
    let task = fs::read_to_string(p.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    for heading in [
        "问题证据",
        "规则依据",
        "允许范围",
        "修复步骤",
        "复检",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(task.contains(heading));
    }
    let again = p.lint(Some(&tool));
    assert_eq!(again["backlog_sync"]["new_findings"], 0);
    assert_eq!(again["backlog_sync"]["failed_reports"], 0);
    fs::write(p.0.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let clean = p.lint(Some(&tool));
    assert!(clean["findings"].as_array().unwrap().is_empty());
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    assert_eq!(clean["delivery_decision"], "not_evaluated");
}

fn synthetic_report(p: &Project) -> Value {
    use codeguard_adapters::GoVetFinding;
    use codeguard_adapters::go_finding_record;
    use sha2::{Digest, Sha256};
    let mut report = p.lint(None);
    report["run_id"] = serde_json::json!("synthetic-go");
    report["native_status"] = serde_json::json!("findings_observed_unverified");
    report["reason"] = serde_json::json!("project_scope_and_policy_unverified");
    report["native_tool_version"] = serde_json::json!("go1.23.4");
    report["tool_sha256"] = serde_json::json!("a".repeat(64));
    report["source_file_count"] = serde_json::json!(1);
    report["module_count"] = serde_json::json!(1);
    report["modules_completed"] = serde_json::json!(1);
    report["module_identities"] = serde_json::json!({".":{
        "manifest_sha256":format!("{:x}",Sha256::digest(fs::read(p.0.join("go.mod")).unwrap())),
        "go_sum_sha256":null
    }});
    report["module_results"] = serde_json::json!([{"root":".","status":"findings_observed_unverified","diagnostic_count":1}]);
    report["native_diagnostic_count"] = serde_json::json!(1);
    let finding = GoVetFinding {
        package: "example.com/tasks".into(),
        native_rule_id: "printf".into(),
        path: "main.go".into(),
        line: 3,
        column: 1,
        message: "synthetic conformance diagnostic".into(),
    };
    report["findings"] = serde_json::json!([go_finding_record(
        ".",
        &finding,
        &fs::read(p.0.join("main.go")).unwrap(),
        0
    )
    .unwrap()]);
    report["source_snapshot_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(
                &serde_json::json!({"main.go": report["findings"][0]["source_sha256"]})
            )
            .unwrap()
        )
    ));
    report
}
fn queue(p: &Project, report: &Value) {
    fs::write(
        p.0.join(".codeguard/reports/synthetic-go.json"),
        serde_json::to_vec(report).unwrap(),
    )
    .unwrap();
}
#[test]
fn inconsistent_go_reports_do_not_create_source_tasks() {
    for kind in [
        "workspace",
        "count",
        "excess_count",
        "path",
        "module",
        "position",
    ] {
        let p = Project::new();
        let mut report = synthetic_report(&p);
        match kind {
            "workspace" => report["workspace_id"] = serde_json::json!("other-workspace"),
            "excess_count" => {
                report["module_results"][0]["diagnostic_count"] = serde_json::json!(u64::MAX)
            }
            "count" => report["module_results"][0]["diagnostic_count"] = serde_json::json!(2),
            "path" => report["findings"][0]["path"] = serde_json::json!("../main.go"),
            "module" => report["findings"][0]["module_root"] = serde_json::json!("nested"),
            "position" => report["findings"][0]["line"] = serde_json::json!(999),
            _ => unreachable!(),
        }
        queue(&p, &report);
        let sync = p.query(&["work", "sync"]);
        assert_eq!(sync["failed_reports"], 1, "{kind}: {sync}");
        assert_eq!(sync["new_findings"], 0);
    }
}
#[test]
fn changed_source_or_module_keeps_old_go_finding_historical() {
    for kind in ["source", "manifest"] {
        let p = Project::new();
        let report = synthetic_report(&p);
        if kind == "source" {
            fs::write(p.0.join("main.go"), "package main\nfunc main() {}\n").unwrap();
        } else {
            fs::write(
                p.0.join("go.mod"),
                "module example.com/changed\n\ngo 1.23\n",
            )
            .unwrap();
        }
        queue(&p, &report);
        let sync = p.query(&["work", "sync"]);
        assert_eq!(sync["failed_reports"], 0, "{sync}");
        assert_eq!(sync["historical_findings"], 1);
        assert_eq!(sync["new_findings"], 0);
    }
}

#[test]
fn go_environment_task_verification_records_failed_recheck() {
    let p = Project::new();
    p.lint(None);
    let brief = p.query(&["next"]);
    let id = brief["repair_brief"]["task_id"].as_str().unwrap();
    let verify = p.query(&["task", "verify", id]);
    assert_eq!(verify["observation"], "still_blocked", "{verify}");
    assert_eq!(verify["event_persisted"], true);
    let next = p.query(&["next"]);
    assert_eq!(
        next["repair_brief"]["verification_observation"], "still_blocked",
        "{next}"
    );
}
#[test]
#[ignore = "requires explicit local Go 1.23.4; no install or network"]
fn go_task_recheck_preserves_evidence_without_unverified_closure() {
    let tool = std::env::var("CODEGUARD_GO_TOOL").unwrap();
    let p = Project::new();
    let scan = p.lint(Some(&tool));
    let id = scan["findings"][0]["finding_id"].as_str().unwrap();
    let verify = || p.query(&["task", "verify", id, "--go-tool", &tool]);
    let present = verify();
    assert_eq!(present["observation"], "still_present", "{present}");
    assert_eq!(present["event_persisted"], true);
    let current = codeguard_cli::next_command::read_task_brief(&p.0, id).unwrap();
    assert_eq!(current["verification_observation"], "still_present");
    fs::write(p.0.join("main.go"), "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\"); fmt.Println(\"after\") }\n").unwrap();
    let changed_anchor = verify();
    assert_eq!(
        changed_anchor["observation"], "rule_coverage_requires_review",
        "{changed_anchor}"
    );
    assert_eq!(changed_anchor["event_persisted"], true);
    fs::write(p.0.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let absent = verify();
    assert_eq!(
        absent["observation"], "candidate_absent_unverified_policy",
        "{absent}"
    );
    assert_eq!(absent["event_persisted"], true);
    assert_eq!(absent["native_scan"]["coverage_proven"], false);
    let absent_brief = codeguard_cli::next_command::read_task_brief(&p.0, id).unwrap();
    assert_eq!(
        absent_brief["verification_observation"],
        "candidate_absent_unverified_policy"
    );
    let manifest = fs::read(p.0.join("go.mod")).unwrap();
    fs::write(
        p.0.join("go.mod"),
        "module example.com/changed\n\ngo 1.23\n",
    )
    .unwrap();
    let changed = codeguard_cli::next_command::read_task_brief(&p.0, id).unwrap();
    assert_eq!(changed["disposition"], "verification_required");
    assert!(changed["step"].as_str().unwrap().contains("模块配置"));
    fs::write(p.0.join("go.mod"), manifest).unwrap();
    let fact: Value = serde_json::from_slice(
        &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(
        p.0.join("main.go"),
        "package main\nfunc main() { missing() }\n",
    )
    .unwrap();
    let broken = verify();
    assert_eq!(broken["observation"], "incomplete", "{broken}");
    assert_eq!(broken["event_persisted"], true);
}

#[test]
#[ignore = "requires explicit local Go 1.23.4; no install or network"]
fn native_build_exclusion_is_not_a_repair_candidate() {
    let tool = std::env::var("CODEGUARD_GO_TOOL").unwrap();
    for kind in ["tag", "cgo", "platform"] {
        let p = Project::new();
        let scan = p.lint(Some(&tool));
        let id = scan["findings"][0]["finding_id"].as_str().unwrap();
        let hidden = match kind {
            "cgo" => "package main\nimport \"C\"\nimport \"fmt\"\nfunc hidden() { fmt.Printf(\"%d\", \"bad\") }\n".to_owned(),
            "platform" => format!("//go:build {}\n\npackage main\nimport \"fmt\"\nfunc hidden() {{ fmt.Printf(\"%d\", \"bad\") }}\n", if std::env::consts::OS == "linux" {"windows"} else {"linux"}),
            _ => "//go:build codeguard_excluded\n\npackage main\nimport \"fmt\"\nfunc hidden() { fmt.Printf(\"%d\", \"bad\") }\n".to_owned(),
        };
        fs::write(p.0.join("main.go"), hidden).unwrap();
        fs::write(p.0.join("active.go"), "package main\nfunc main() {}\n").unwrap();
        let verify = p.query(&["task", "verify", id, "--go-tool", &tool]);
        assert_eq!(
            verify["observation"], "rule_coverage_requires_review",
            "{kind}: {verify}"
        );
        assert_eq!(verify["native_scan"]["task_scope"]["status"], "excluded");
        assert_eq!(verify["event_persisted"], true);
        let brief = codeguard_cli::next_command::read_task_brief(&p.0, id).unwrap();
        assert!(brief["step"].as_str().unwrap().contains("构建"), "{brief}");
        let fact: Value = serde_json::from_slice(
            &fs::read(p.0.join(format!(".codeguard/findings/{id}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        fs::write(p.0.join("main.go"), "package main\nfunc updated() {}\n").unwrap();
        let stale = codeguard_cli::next_command::read_task_brief(&p.0, id).unwrap();
        assert_eq!(stale["disposition"], "verification_required", "{stale}");
    }
}
