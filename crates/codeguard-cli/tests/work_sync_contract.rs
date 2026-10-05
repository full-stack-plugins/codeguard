use codeguard_adapters::bundled_ruff_rulepack;
use codeguard_runtime::TaskFileLock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn published_sync_and_record_schemas_cannot_claim_quality_allow() {
    for source in [
        include_str!("../../../schemas/work-sync-preview.schema.json"),
        include_str!("../../../schemas/work-sync-preview-v0.3.schema.json"),
        include_str!("../../../schemas/finding-record.schema.json"),
        include_str!("../../../schemas/finding-observed-event.schema.json"),
        include_str!("../../../schemas/blocker-record.schema.json"),
        include_str!("../../../schemas/blocker-observed-event.schema.json"),
        include_str!("../../../schemas/consumed-report-marker.schema.json"),
        include_str!("../../../schemas/local-finding-observation.schema.json"),
        include_str!("../../../schemas/local-blocker-observation.schema.json"),
    ] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert!(!source.contains("\"allow\""));
    }
}

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-work-sync-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), "import os\n").unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        Self(root)
    }

    fn workspace_id(&self) -> String {
        let value: Value =
            serde_json::from_slice(&fs::read(self.0.join(".codeguard/workspace.json")).unwrap())
                .unwrap();
        value["workspace_id"].as_str().unwrap().into()
    }

    fn report(&self, run_id: &str, workspace_id: &str) -> Value {
        let fingerprint = "a".repeat(64);
        let source_sha256 = format!(
            "{:x}",
            Sha256::digest(fs::read(self.0.join("app.py")).unwrap())
        );
        json!({
            "schema_version":"0.4.0", "report_type":"python_lint_feedback",
            "operation":"lint", "language":"python", "run_id":run_id,
            "workspace_binding":"bound", "workspace_id":workspace_id,
            "command_status":"incomplete", "delivery_decision":"not_evaluated",
            "tool_approval":"unverified",
            "files":[{
                "path":"app.py", "source_sha256":source_sha256,
                "run_status":"findings", "recheck_cwd":self.0,
                "recheck_argv":["ruff","check","app.py"],
                "findings":[{"finding_id":format!("CG-{}", &fingerprint[..32]),
                    "finding_fingerprint":fingerprint, "path":"app.py", "rule_id":"F401", "line":1}]
            }]
        })
    }

    fn write_report(&self, run_id: &str, report: &Value) {
        fs::write(
            self.0.join(format!(".codeguard/reports/{run_id}.json")),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }

    fn sync(&self) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["work", "sync", self.0.to_str().unwrap(), "--format=json"])
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
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn sync_rejects_a_concurrent_import_before_writing_records() {
    let project = Project::new();
    let workspace_id = project.workspace_id();
    project.write_report("run-one", &project.report("run-one", &workspace_id));
    let state = project.0.join(".codeguard/state");
    let held = TaskFileLock::acquire(&state.join("work-sync.lock")).unwrap();
    let (exit, blocked) = project.sync();
    assert_eq!(exit, 3);
    assert_eq!(blocked["schema_version"], "0.2.0");
    assert_eq!(blocked["reason"], "work_sync_busy");
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/findings"))
            .unwrap()
            .count(),
        0
    );
    assert!(!state.join("consumed/run-one.json").exists());
    drop(held);

    let (_, recovered) = project.sync();
    assert_eq!(recovered["imported_reports"], 1);
    assert_eq!(recovered["new_findings"], 1);
    let (_, repeated) = project.sync();
    assert_eq!(repeated["already_consumed_reports"], 1);
    assert_eq!(repeated["new_findings"], 0);
}

#[test]
fn first_finding_import_resumes_from_each_persisted_boundary() {
    for boundary in ["task", "fact", "observation", "event"] {
        let project = Project::new();
        let workspace_id = project.workspace_id();
        project.write_report("run-one", &project.report("run-one", &workspace_id));
        assert_eq!(project.sync().1["new_findings"], 1);

        let finding_id = format!("CG-{}", "a".repeat(32));
        let fact = project
            .0
            .join(format!(".codeguard/findings/{finding_id}/finding.json"));
        let task = project.0.join(format!(".codeguard/tasks/{finding_id}.md"));
        let event = project.0.join(format!(
            ".codeguard/findings/{finding_id}/events/run-one.json"
        ));
        let observation = project.0.join(format!(
            ".codeguard/state/observations/{finding_id}/run-one.json"
        ));
        let marker = project.0.join(".codeguard/state/consumed/run-one.json");
        let original_fact = fs::read(&fact).unwrap();
        let original_task = fs::read(&task).unwrap();
        let original_event = fs::read(&event).unwrap();
        let original_observation = fs::read(&observation).unwrap();
        fs::remove_file(&marker).unwrap();
        if boundary != "event" {
            fs::remove_file(&event).unwrap();
        }
        if matches!(boundary, "task" | "fact") {
            fs::remove_file(&observation).unwrap();
        }
        if boundary == "task" {
            fs::remove_file(&fact).unwrap();
        }

        let (_, recovered) = project.sync();
        assert_eq!(recovered["failed_reports"], 0, "{boundary}: {recovered}");
        assert_eq!(fs::read(&fact).unwrap(), original_fact, "{boundary}");
        assert_eq!(fs::read(&task).unwrap(), original_task, "{boundary}");
        assert_eq!(fs::read(&event).unwrap(), original_event, "{boundary}");
        assert_eq!(
            fs::read(&observation).unwrap(),
            original_observation,
            "{boundary}"
        );
        assert!(marker.is_file(), "{boundary}");
        assert_eq!(fs::read_dir(event.parent().unwrap()).unwrap().count(), 1);
    }
}

#[test]
fn blocker_fact_without_observation_or_event_is_replayed_before_cursor() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            "/nonexistent/codeguard-ruff",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let scan: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(scan["backlog_sync"]["new_blockers"], 1);
    let run_id = scan["run_id"].as_str().unwrap();
    let blocker_id = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .map(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .trim_end_matches(".md")
                .to_owned()
        })
        .find(|id| {
            let fact: Value = serde_json::from_slice(
                &fs::read(
                    project
                        .0
                        .join(format!(".codeguard/findings/{id}/finding.json")),
                )
                .unwrap(),
            )
            .unwrap();
            fact["reason_code"] == "ruff_tool_not_found"
        })
        .unwrap();
    let fact = project
        .0
        .join(format!(".codeguard/findings/{blocker_id}/finding.json"));
    let event = project.0.join(format!(
        ".codeguard/findings/{blocker_id}/events/{run_id}.json"
    ));
    let observation = project.0.join(format!(
        ".codeguard/state/observations/{blocker_id}/{run_id}.json"
    ));
    let marker = project
        .0
        .join(format!(".codeguard/state/consumed/{run_id}.json"));
    let original_fact = fs::read(&fact).unwrap();
    let original_event = fs::read(&event).unwrap();
    let original_observation = fs::read(&observation).unwrap();
    fs::remove_file(&event).unwrap();
    fs::remove_file(&observation).unwrap();
    fs::remove_file(&marker).unwrap();

    let (_, recovered) = project.sync();
    assert_eq!(recovered["failed_reports"], 0, "{recovered}");
    assert_eq!(fs::read(&fact).unwrap(), original_fact);
    assert_eq!(fs::read(&event).unwrap(), original_event);
    assert_eq!(fs::read(&observation).unwrap(), original_observation);
    assert!(marker.is_file());
    assert_eq!(fs::read_dir(event.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn repeat_reports_keep_one_finding_and_crash_retry_restores_marker() {
    let project = Project::new();
    let id = project.workspace_id();
    project.write_report("run-one", &project.report("run-one", &id));
    let (exit, first) = project.sync();
    assert_eq!(exit, 3);
    assert_eq!(first["new_findings"], 1);
    assert_eq!(first["delivery_decision"], "not_evaluated");
    let finding = project.0.join(format!(
        ".codeguard/findings/CG-{}/finding.json",
        "a".repeat(32)
    ));
    let task = project
        .0
        .join(format!(".codeguard/tasks/CG-{}.md", "a".repeat(32)));
    let before_fact = fs::read(&finding).unwrap();
    let before_task = fs::read(&task).unwrap();
    assert!(!String::from_utf8_lossy(&before_task).contains("import os"));
    assert_eq!(project.sync().1["already_consumed_reports"], 1);
    project.write_report("run-two", &project.report("run-two", &id));
    let (_, second) = project.sync();
    assert_eq!(second["imported_reports"], 1);
    assert_eq!(second["new_findings"], 0);
    assert_eq!(fs::read(&finding).unwrap(), before_fact);
    assert_eq!(fs::read(&task).unwrap(), before_task);
    let events = finding.parent().unwrap().join("events");
    assert!(events.join("run-one.json").is_file());
    assert!(!events.join("run-two.json").exists());
    assert_eq!(fs::read_dir(&events).unwrap().count(), 1);
    let observations = project.0.join(format!(
        ".codeguard/state/observations/CG-{}",
        "a".repeat(32)
    ));
    for run in ["run-one", "run-two"] {
        let observation: Value =
            serde_json::from_slice(&fs::read(observations.join(format!("{run}.json"))).unwrap())
                .unwrap();
        assert_eq!(observation["finding_id"], format!("CG-{}", "a".repeat(32)));
        assert_eq!(observation["run_id"], run);
        assert_eq!(observation["record_type"], "local_finding_observation");
        assert_eq!(observation["authority"], "local_unverified");
        assert!(
            observation["report_sha256"]
                .as_str()
                .is_some_and(|digest| digest.len() == 64)
        );
        assert_eq!(
            observation["source_sha256"],
            project.report(run, &id)["files"][0]["source_sha256"]
        );
    }
    for index in 3..=10 {
        let run = format!("run-{index}");
        project.write_report(&run, &project.report(&run, &id));
        assert_eq!(project.sync().1["new_findings"], 0);
        assert!(observations.join(format!("{run}.json")).is_file());
    }
    assert_eq!(fs::read_dir(&events).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&observations).unwrap().count(), 10);
    fs::remove_file(project.0.join(".codeguard/state/consumed/run-one.json")).unwrap();
    let (_, recovered) = project.sync();
    assert_eq!(recovered["imported_reports"], 1);
    assert_eq!(recovered["new_findings"], 0);
    assert!(
        project
            .0
            .join(".codeguard/state/consumed/run-one.json")
            .exists()
    );
    assert_eq!(fs::read(&task).unwrap(), before_task);
    assert_eq!(fs::read_dir(&events).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&observations).unwrap().count(), 10);
}

#[test]
fn reappearance_after_verification_emits_one_event_then_keeps_local_observations() {
    let project = Project::new();
    let workspace_id = project.workspace_id();
    let finding_id = format!("CG-{}", "a".repeat(32));
    let events = project
        .0
        .join(format!(".codeguard/findings/{finding_id}/events"));
    let observations = project
        .0
        .join(format!(".codeguard/state/observations/{finding_id}"));
    project.write_report("lint-1-100", &project.report("lint-1-100", &workspace_id));
    assert_eq!(project.sync().1["new_findings"], 1);
    fs::write(events.join("verify-lint-1-200.json"), b"{}\n").unwrap();
    for sequence in [300, 400] {
        let run = format!("lint-1-{sequence}");
        project.write_report(&run, &project.report(&run, &workspace_id));
        assert_eq!(project.sync().1["new_findings"], 0);
        assert!(observations.join(format!("{run}.json")).is_file());
    }
    assert!(events.join("lint-1-300.json").is_file());
    assert!(!events.join("lint-1-400.json").exists());
}

#[test]
fn new_local_report_requires_completed_artifact_hashes_but_old_report_still_imports() {
    let invalid = Project::new();
    let mut report = invalid.report("run-new-invalid", &invalid.workspace_id());
    report["schema_version"] = json!("0.5.0");
    invalid.write_report("run-new-invalid", &report);
    let (_, result) = invalid.sync();
    assert_eq!(result["failed_reports"], 1);
    assert_eq!(result["new_findings"], 0);

    let valid = Project::new();
    let mut report = valid.report("run-new-valid", &valid.workspace_id());
    report["schema_version"] = json!("0.5.0");
    report["files"][0]["tool_sha256"] = json!("b".repeat(64));
    report["files"][0]["config_sha256"] = json!("c".repeat(64));
    valid.write_report("run-new-valid", &report);
    let (_, result) = valid.sync();
    assert_eq!(result["failed_reports"], 0);
    assert_eq!(result["new_findings"], 1);

    let old = Project::new();
    old.write_report("run-old", &old.report("run-old", &old.workspace_id()));
    let (_, result) = old.sync();
    assert_eq!(result["failed_reports"], 0);
    assert_eq!(result["new_findings"], 1);
}

#[test]
fn mapped_rulepack_digest_is_observation_only_and_tampering_rejects_new_report() {
    let pack = bundled_ruff_rulepack().unwrap();
    let valid = Project::new();
    let mut report = valid.report("run-mapped", &valid.workspace_id());
    report["schema_version"] = json!("0.6.0");
    report["rulepack_approval"] = json!("unverified");
    report["native_tool_version"] = json!("ruff 0.16.8");
    report["files"][0]["tool_sha256"] = json!("b".repeat(64));
    report["files"][0]["config_sha256"] = json!("c".repeat(64));
    report["files"][0]["findings"][0]["codeguard_rule_id"] = json!("python.ruff.F401");
    report["files"][0]["findings"][0]["rulepack_sha256"] = json!(pack.sha256);
    report["files"][0]["findings"][0]["rulepack_status"] = json!("candidate_unapproved");
    valid.write_report("run-mapped", &report);
    let (_, result) = valid.sync();
    assert_eq!(result["failed_reports"], 0);
    assert_eq!(result["new_findings"], 1);

    let invalid = Project::new();
    report["run_id"] = json!("run-tampered");
    report["workspace_id"] = json!(invalid.workspace_id());
    report["files"][0]["recheck_cwd"] = json!(invalid.0);
    report["files"][0]["findings"][0]["rulepack_sha256"] = json!("d".repeat(64));
    invalid.write_report("run-tampered", &report);
    let (_, result) = invalid.sync();
    assert_eq!(result["failed_reports"], 1);
    assert_eq!(result["new_findings"], 0);
}

#[test]
fn version_seven_requires_bounded_native_settings_without_claiming_coverage() {
    let pack = bundled_ruff_rulepack().unwrap();
    for (variant, expected_failures) in [("valid", 0), ("missing", 1), ("forged_coverage", 1)] {
        let project = Project::new();
        let run_id = format!("run-settings-{variant}");
        let mut report = project.report(&run_id, &project.workspace_id());
        report["schema_version"] = json!("0.7.0");
        report["rulepack_approval"] = json!("unverified");
        report["native_tool_version"] = json!("ruff 0.16.8");
        report["files"][0]["tool_sha256"] = json!("b".repeat(64));
        report["files"][0]["config_sha256"] = json!("c".repeat(64));
        report["files"][0]["findings"][0]["codeguard_rule_id"] = json!("python.ruff.F401");
        report["files"][0]["findings"][0]["rulepack_sha256"] = json!(pack.sha256.clone());
        report["files"][0]["findings"][0]["rulepack_status"] = json!("candidate_unapproved");
        report["files"][0]["rule_settings"] = json!({
            "settings_sha256":"d".repeat(64),
            "globally_enabled_mapped_rules":["F401"],
            "per_file_ignores_present":false,
            "coverage_proven":false
        });
        if variant == "missing" {
            report["files"][0]
                .as_object_mut()
                .unwrap()
                .remove("rule_settings");
        } else if variant == "forged_coverage" {
            report["files"][0]["rule_settings"]["coverage_proven"] = json!(true);
        }
        project.write_report(&run_id, &report);
        let (_, result) = project.sync();
        assert_eq!(result["failed_reports"], expected_failures, "{variant}");
        assert_eq!(result["new_findings"], 1 - expected_failures, "{variant}");
    }
}

#[test]
fn version_eight_requires_separate_bounded_suppression_observation() {
    let pack = bundled_ruff_rulepack().unwrap();
    for (variant, expected_failures, expected_findings) in [
        ("valid", 0, 1),
        ("missing", 1, 0),
        ("inconsistent", 1, 0),
        ("suppressed_without_diff", 1, 0),
        ("suppressed_hides_finding", 1, 0),
        ("passed_hides_suppression", 1, 0),
        ("clean_suppressed", 0, 0),
    ] {
        let project = Project::new();
        let run_id = format!("run-audit-{variant}");
        let mut report = project.report(&run_id, &project.workspace_id());
        report["schema_version"] = json!("0.8.0");
        report["rulepack_approval"] = json!("unverified");
        report["native_tool_version"] = json!("ruff 0.16.8");
        report["files"][0]["tool_sha256"] = json!("b".repeat(64));
        report["files"][0]["config_sha256"] = json!("c".repeat(64));
        report["files"][0]["findings"][0]["codeguard_rule_id"] = json!("python.ruff.F401");
        report["files"][0]["findings"][0]["rulepack_sha256"] = json!(pack.sha256.clone());
        report["files"][0]["findings"][0]["rulepack_status"] = json!("candidate_unapproved");
        report["files"][0]["rule_settings"] = json!({
            "settings_sha256":"d".repeat(64),
            "globally_enabled_mapped_rules":["F401"],
            "per_file_ignores_present":false,
            "coverage_proven":false
        });
        report["files"][0]["suppression_audit"] = json!({
            "audit_sha256":"e".repeat(64),
            "suppressed_diagnostic_count":0,
            "suppressed_rule_ids":[],
            "scope":"source_comments_only"
        });
        if variant == "missing" {
            report["files"][0]
                .as_object_mut()
                .unwrap()
                .remove("suppression_audit");
        } else if variant == "inconsistent" {
            report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"] = json!(1);
        } else if variant == "suppressed_without_diff" {
            report["files"][0]["run_status"] = json!("suppressed");
            report["files"][0]["findings"] = json!([]);
        } else if variant == "suppressed_hides_finding" {
            report["files"][0]["run_status"] = json!("suppressed");
            report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"] = json!(1);
            report["files"][0]["suppression_audit"]["suppressed_rule_ids"] = json!(["F401"]);
        } else if variant == "passed_hides_suppression" {
            report["files"][0]["run_status"] = json!("passed");
            report["files"][0]["findings"] = json!([]);
            report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"] = json!(1);
            report["files"][0]["suppression_audit"]["suppressed_rule_ids"] = json!(["F401"]);
        } else if variant == "clean_suppressed" {
            report["files"][0]["run_status"] = json!("suppressed");
            report["files"][0]["findings"] = json!([]);
            report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"] = json!(1);
            report["files"][0]["suppression_audit"]["suppressed_rule_ids"] = json!(["F401"]);
        }
        project.write_report(&run_id, &report);
        let (_, result) = project.sync();
        assert_eq!(result["failed_reports"], expected_failures, "{variant}");
        assert_eq!(result["new_findings"], expected_findings, "{variant}");
    }
}

#[test]
fn stale_report_is_historical_and_bad_report_does_not_block_valid_one() {
    let project = Project::new();
    let id = project.workspace_id();
    project.write_report("run-stale", &project.report("run-stale", &id));
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let (_, stale) = project.sync();
    assert_eq!(stale["historical_findings"], 1);
    assert_eq!(stale["new_findings"], 0);
    assert!(
        !project
            .0
            .join(format!(".codeguard/tasks/CG-{}.md", "a".repeat(32)))
            .exists()
    );
    project.write_report("run-bad", &project.report("run-bad", "ws-invalid"));
    project.write_report("run-good", &project.report("run-good", &id));
    let (_, mixed) = project.sync();
    assert_eq!(mixed["failed_reports"], 1);
    assert_eq!(mixed["imported_reports"], 1);
    assert_eq!(mixed["new_findings"], 1);
    assert_eq!(mixed["command_status"], "incomplete");
}

#[test]
fn same_run_id_with_changed_report_digest_is_rejected() {
    let project = Project::new();
    let id = project.workspace_id();
    project.write_report("run-one", &project.report("run-one", &id));
    assert_eq!(project.sync().1["imported_reports"], 1);
    let mut changed = project.report("run-one", &id);
    changed["extra"] = json!("changed");
    project.write_report("run-one", &changed);
    let (_, result) = project.sync();
    assert_eq!(result["failed_reports"], 1);
    assert_eq!(result["already_consumed_reports"], 0);
    assert_eq!(result["delivery_decision"], "not_evaluated");
}

#[test]
fn missing_tool_groups_two_files_into_one_actionable_blocker() {
    let project = Project::new();
    fs::write(project.0.join("other.py"), "import sys\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let scan = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "python",
                project.0.to_str().unwrap(),
                "--ruff-tool",
                "/nonexistent/codeguard-ruff",
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let first = scan();
    assert_eq!(first["files"].as_array().unwrap().len(), 2);
    assert!(first["files"].as_array().unwrap().iter().all(|file| {
        file["run_status"] == "incomplete" && file["reason"] == "ruff_tool_not_found"
    }));
    assert_eq!(first["backlog_sync"]["new_findings"], 0);
    assert_eq!(first["backlog_sync"]["new_blockers"], 1);
    let task = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(task.len(), 1);
    let task_before = fs::read(&task[0]).unwrap();
    let text = String::from_utf8_lossy(&task_before);
    assert!(text.contains("工具锁"));
    assert!(text.contains("不是源码违规"));
    assert!(!text.contains("移除该导入"));
    let id = task[0].file_stem().unwrap().to_str().unwrap();
    let fact: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["kind"], "blocker");
    assert_eq!(fact["first_affected_paths"].as_array().unwrap().len(), 2);
    let second = scan();
    assert_eq!(second["backlog_sync"]["new_blockers"], 0);
    assert_eq!(fs::read(&task[0]).unwrap(), task_before);
    assert_eq!(
        fs::read_dir(project.0.join(format!(".codeguard/findings/{id}/events")))
            .unwrap()
            .count(),
        1
    );
    let observations = project
        .0
        .join(format!(".codeguard/state/observations/{id}"));
    assert_eq!(fs::read_dir(&observations).unwrap().count(), 2);
    for scan in [&first, &second] {
        let run_id = scan["run_id"].as_str().unwrap();
        let observation: Value =
            serde_json::from_slice(&fs::read(observations.join(format!("{run_id}.json"))).unwrap())
                .unwrap();
        assert_eq!(observation["record_type"], "local_blocker_observation");
        assert_eq!(observation["blocker_id"], id);
    }
    fs::remove_file(&task[0]).unwrap();
    let recovered = project.sync().1;
    assert_eq!(recovered["restored_task_projections"], 1, "{recovered}");
    assert_eq!(recovered["new_blockers"], 0);
    let projection = fs::read_to_string(&task[0]).unwrap();
    assert!(projection.contains("ruff_tool_not_found"));
    assert!(projection.contains("blocker"));
    assert!(!projection.contains("移除该导入"));
    let unchanged: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(unchanged, fact);
}

#[test]
fn missing_config_creates_preparation_task_without_claiming_violation() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert_eq!(report["backlog_sync"]["new_blockers"], 1);
    let tasks = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let task = tasks
        .iter()
        .find(|path| {
            let id = path.file_stem().unwrap().to_str().unwrap();
            let fact: Value = serde_json::from_slice(
                &fs::read(
                    project
                        .0
                        .join(format!(".codeguard/findings/{id}/finding.json")),
                )
                .unwrap(),
            )
            .unwrap();
            fact["reason_code"] == "project_ruff_config_not_found"
        })
        .unwrap();
    if cfg!(feature = "wasm-precheck") {
        assert_eq!(tasks.len(), 2);
    } else {
        assert_eq!(tasks.len(), 1);
    }
    let text = fs::read_to_string(task).unwrap();
    assert!(text.contains("确认项目是否要求 Ruff"));
    assert!(text.contains("修订项目质量策略"));
    assert!(text.contains("原检查器对受阻义务完成有效复检"));
}

#[test]
fn same_reason_in_different_build_roots_is_not_merged() {
    let project = Project::new();
    let id = project.workspace_id();
    let mut report = project.report("run-modules", &id);
    report["checker_configurations"] = json!([
        {"checker_id":"python.ruff", "build_root":"service_a"},
        {"checker_id":"python.ruff", "build_root":"service_b"}
    ]);
    report["files"] = json!([
        {"path":"service_a/app.py", "run_status":"incomplete",
         "reason":"ruff_tool_not_found", "recheck_cwd":project.0,
         "recheck_argv":["ruff", "check", "service_a/app.py"], "findings":[]},
        {"path":"service_b/app.py", "run_status":"incomplete",
         "reason":"ruff_tool_not_found", "recheck_cwd":project.0,
         "recheck_argv":["ruff", "check", "service_b/app.py"], "findings":[]}
    ]);
    project.write_report("run-modules", &report);
    let (_, result) = project.sync();
    assert_eq!(result["new_blockers"], 2);
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn native_ruff_scan_queues_and_syncs_one_stable_task() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let scan = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "python",
                project.0.to_str().unwrap(),
                "--ruff-tool",
                &tool,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let first = scan();
    assert_eq!(first["files"][0]["run_status"], "findings");
    assert_eq!(first["backlog_status"], "synced_partial");
    assert_eq!(first["backlog_sync"]["new_findings"], 1);
    let id = first["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    let (_, synced) = project.sync();
    assert_eq!(synced["new_findings"], 0);
    assert_eq!(synced["already_consumed_reports"], 1);
    let task_path = project.0.join(format!(".codeguard/tasks/{id}.md"));
    let task = fs::read(&task_path).unwrap();
    assert!(!String::from_utf8_lossy(&task).contains("os imported but unused"));
    let second = scan();
    assert_eq!(second["files"][0]["findings"][0]["finding_id"], id);
    assert_eq!(second["backlog_sync"]["new_findings"], 0);
    let (_, synced) = project.sync();
    assert_eq!(synced["new_findings"], 0);
    assert_eq!(synced["already_consumed_reports"], 2);
    assert_eq!(fs::read(task_path).unwrap(), task);
}

#[test]
#[ignore = "requires pinned native Ruff; run with CODEGUARD_RUFF_BIN"]
fn backlog_failure_keeps_native_finding_visible_without_claiming_task_creation() {
    let project = Project::new();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    fs::remove_dir(project.0.join(".codeguard/reports")).unwrap();
    fs::write(project.0.join(".codeguard/reports"), "blocked").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            &tool,
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    assert_eq!(report["backlog_status"], "backlog_update_failed");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(
        fs::read_dir(project.0.join(".codeguard/tasks"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn consumed_finding_recovers_missing_projection_without_changing_evidence() {
    let p = Project::new();
    p.write_report("run-one", &p.report("run-one", &p.workspace_id()));
    assert_eq!(p.sync().1["new_findings"], 1);
    let id = format!("CG-{}", "a".repeat(32));
    let task = p.0.join(format!(".codeguard/tasks/{id}.md"));
    let paths = [
        p.0.join(format!(".codeguard/findings/{id}/finding.json")),
        p.0.join(format!(".codeguard/findings/{id}/events/run-one.json")),
        p.0.join(".codeguard/state/consumed/run-one.json"),
    ];
    let before: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    fs::remove_file(&task).unwrap();
    let r = p.sync().1;
    assert_eq!(r["restored_task_projections"], 1, "{r}");
    assert_eq!(r["new_findings"], 0);
    assert_eq!(r["delivery_decision"], "not_evaluated");
    let text = fs::read_to_string(&task).unwrap();
    for heading in [
        "问题证据",
        "规则依据",
        "允许修改的范围",
        "修复步骤",
        "复检命令",
        "历史尝试",
        "关闭条件",
    ] {
        assert!(text.contains(heading), "{text}");
    }
    assert!(!text.contains(p.0.to_str().unwrap()));
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).unwrap(), expected);
    }
    fs::write(&task, "人工备注\n- [x] 我认为已修复\n").unwrap();
    let again = p.sync().1;
    assert_eq!(again["schema_version"], "0.2.0");
    assert_eq!(
        fs::read_to_string(&task).unwrap(),
        "人工备注\n- [x] 我认为已修复\n"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("next")
        .arg(&p.0)
        .arg("--format=json")
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(next["repair_brief"]["task_id"], id);
    assert_eq!(next["delivery_decision"], "not_evaluated");
}

#[test]
fn projection_recovery_rejects_wrong_workspace_fact_before_creating_markdown() {
    let p = Project::new();
    p.write_report("run-one", &p.report("run-one", &p.workspace_id()));
    p.sync();
    let id = format!("CG-{}", "a".repeat(32));
    let task = p.0.join(format!(".codeguard/tasks/{id}.md"));
    fs::remove_file(&task).unwrap();
    let fact_path = p.0.join(format!(".codeguard/findings/{id}/finding.json"));
    let mut fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    fact["workspace_id"] = json!("ws-00000000000000000000000000000000");
    fs::write(&fact_path, serde_json::to_vec(&fact).unwrap()).unwrap();
    let r = p.sync().1;
    assert_eq!(r["command_status"], "incomplete", "{r}");
    assert!(!task.exists());
}

#[cfg(unix)]
#[test]
fn projection_recovery_never_follows_a_task_symlink() {
    let p = Project::new();
    p.write_report("run-one", &p.report("run-one", &p.workspace_id()));
    p.sync();
    let task =
        p.0.join(format!(".codeguard/tasks/CG-{}.md", "a".repeat(32)));
    fs::remove_file(&task).unwrap();
    let outside = p.0.join("private-note.md");
    fs::write(&outside, "人工数据").unwrap();
    std::os::unix::fs::symlink(&outside, &task).unwrap();
    assert_eq!(p.sync().1["command_status"], "incomplete");
    assert_eq!(fs::read_to_string(&outside).unwrap(), "人工数据");
    assert!(
        fs::symlink_metadata(&task)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn new_scan_and_deleted_projection_recover_in_one_sync_without_report_failure() {
    let p = Project::new();
    p.write_report("run-one", &p.report("run-one", &p.workspace_id()));
    p.sync();
    let task =
        p.0.join(format!(".codeguard/tasks/CG-{}.md", "a".repeat(32)));
    fs::remove_file(task).unwrap();
    p.write_report("run-two", &p.report("run-two", &p.workspace_id()));
    let r = p.sync().1;
    assert_eq!(r["failed_reports"], 0, "{r}");
    assert_eq!(r["restored_task_projections"], 1);
    assert_eq!(r["imported_reports"], 1);
    assert_eq!(r["new_findings"], 0);
}
