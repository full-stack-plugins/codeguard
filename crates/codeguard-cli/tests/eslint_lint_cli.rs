#![cfg(unix)]
use serde_json::Value;
use std::process::Command;
#[test]
fn missing_eslint_context_returns_preparation_without_quality_allow() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "typescript", ".", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["report_type"], "eslint_local_feedback");
    assert_eq!(report["reason"], "eslint_execution_context_missing");
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["findings"].as_array().unwrap().is_empty());
}
#[test]
fn invalid_or_duplicate_eslint_options_are_usage_errors() {
    for tail in [
        vec!["--fix"],
        vec!["--timeout", "0s"],
        vec!["--format", "json", "--format", "human"],
        vec!["--node-tool", "relative"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "typescript", "."])
            .args(tail)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Project(std::path::PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn project() -> Project {
    use std::os::unix::fs::DirBuilderExt;
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-eslint-cli-{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    std::fs::write(root.join("app.js"), "debugger;").unwrap();
    std::fs::write(
        root.join("eslint.config.cjs"),
        "module.exports=[{rules:{'no-debugger':'error'}}]",
    )
    .unwrap();
    Project(root)
}
fn lint(
    project: &Project,
    node: &std::path::Path,
    entry: &std::path::Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("app.js").to_str().unwrap(),
            "--node-tool",
            node.to_str().unwrap(),
            "--eslint-entry",
            entry.to_str().unwrap(),
            "--config",
            project.0.join("eslint.config.cjs").to_str().unwrap(),
            "--cwd",
            project.0.to_str().unwrap(),
            "--eslint-version",
            "10.11.0",
            "--timeout",
            "60s",
            "--format",
            "json",
        ])
        .args(if project.0.join("codeguard").is_dir() {
            vec!["--workspace", project.0.to_str().unwrap()]
        } else {
            vec![]
        })
        .output()
        .unwrap()
}
#[test]
fn controlled_cli_findings_are_feedback_without_leaking_native_message_or_allow() {
    use std::os::unix::fs::PermissionsExt;
    let project = project();
    let node = project.0.join("node");
    let entry = project.0.join("eslint.cjs");
    std::fs::write(&entry, "fixture").unwrap();
    let native = serde_json::json!([{"filePath":project.0.join("app.js"),"messages":[{"ruleId":"no-debugger","severity":2,"message":"PRIVATE_NATIVE_MESSAGE","line":1,"column":1}],"suppressedMessages":[],"errorCount":1,"warningCount":0,"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
    std::fs::write(&node,format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\nprintf '%s' '{}' > \"$report\"\nexit 1\n",native)).unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = lint(&project, &node, &entry);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_coherent"], true);
    assert_eq!(report["findings"][0]["rule_id"], "no-debugger");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["workbench_status"], "not_connected");
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("PRIVATE_NATIVE_MESSAGE")
    );
    assert!(!project.0.join("codeguard").exists());
}
#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0"]
fn native_eslint_cli_finding_and_repaired_file_never_claim_project_delivery() {
    let project = project();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    for repaired in [false, true] {
        if repaired {
            std::fs::write(project.0.join("app.js"), "console.log('repaired');").unwrap();
        }
        let output = lint(
            &project,
            std::path::Path::new(&node),
            std::path::Path::new(&entry),
        );
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["local_coherent"], true, "{report}");
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert!(report["workbench"]["next_error"].is_null(), "{report}");
        assert_eq!(
            report["findings"].as_array().unwrap().len(),
            usize::from(!repaired)
        );
        assert_eq!(
            report["workbench"]["next"]["repair_brief"]["disposition"],
            if repaired {
                "verification_required"
            } else {
                "actionable"
            },
            "{report}"
        );
        assert_eq!(report["coverage_proven"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        let id = report["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap();
        let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "verify",
                id,
                project.0.to_str().unwrap(),
                "--node-tool",
                std::path::Path::new(&node).to_str().unwrap(),
                "--eslint-entry",
                std::path::Path::new(&entry).to_str().unwrap(),
                "--eslint-version",
                "10.11.0",
                "--config",
                project.0.join("eslint.config.cjs").to_str().unwrap(),
                "--cwd",
                project.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(verified.status.code(), Some(3));
        let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
        assert_eq!(
            verified["observation"],
            if repaired {
                "candidate_absent_unverified_policy"
            } else {
                "still_present"
            },
            "{verified}"
        );
        assert_eq!(verified["event_persisted"], true, "{verified}");
        assert_eq!(verified["delivery_decision"], "not_evaluated");
    }
}

#[test]
fn initialized_eslint_workbench_reuses_task_and_preserves_manual_notes() {
    use std::os::unix::fs::PermissionsExt;
    let project = project();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let node = project.0.join("node");
    let entry = project.0.join("eslint.cjs");
    std::fs::write(&entry, "fixture").unwrap();
    let native = serde_json::json!([{"filePath":project.0.join("app.js"),"messages":[{"ruleId":"@typescript-eslint/no-unused-vars","severity":2,"message":"private diagnostic","line":1,"column":1}],"suppressedMessages":[],"errorCount":1,"warningCount":0,"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
    std::fs::write(&node,format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\nprintf '%s' '{}' > \"$report\"\nexit 1\n",native)).unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut task_path = None;
    for round in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                project.0.join("app.js").to_str().unwrap(),
                "--node-tool",
                node.to_str().unwrap(),
                "--eslint-entry",
                entry.to_str().unwrap(),
                "--eslint-version",
                "10.11.0",
                "--config",
                project.0.join("eslint.config.cjs").to_str().unwrap(),
                "--cwd",
                project.0.to_str().unwrap(),
                "--workspace",
                project.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert!(report["workbench"]["next_error"].is_null(), "{report}");
        assert_eq!(
            report["workbench"]["new_findings"],
            if round == 0 { 1 } else { 0 }
        );
        assert_eq!(
            report["workbench"]["next"]["repair_brief"]["checker_id"], "node.eslint",
            "{report}"
        );
        assert_eq!(
            report["workbench"]["next"]["repair_brief"]["disposition"], "actionable",
            "{report}"
        );
        let directories: Vec<_> = std::fs::read_dir(project.0.join("codeguard/findings"))
            .unwrap()
            .collect();
        assert_eq!(directories.len(), 1);
        let fact: Value = serde_json::from_slice(
            &std::fs::read(directories[0].as_ref().unwrap().path().join("finding.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["checker_id"], "node.eslint");
        assert_eq!(fact["state"], "open");
        if round == 0 {
            let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args([
                    "task",
                    "verify",
                    fact["id"].as_str().unwrap(),
                    project.0.to_str().unwrap(),
                    "--format",
                    "json",
                ])
                .output()
                .unwrap();
            assert_eq!(verified.status.code(), Some(3));
            let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
            assert_eq!(
                verified["native_scan"]["report_type"], "eslint_task_recheck",
                "{verified}"
            );
            assert_eq!(verified["observation"], "incomplete");
            assert_eq!(verified["event_persisted"], true);
            let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args([
                    "task",
                    "verify",
                    fact["id"].as_str().unwrap(),
                    project.0.to_str().unwrap(),
                    "--node-tool",
                    node.to_str().unwrap(),
                    "--eslint-entry",
                    entry.to_str().unwrap(),
                    "--eslint-version",
                    "10.11.0",
                    "--config",
                    project.0.join("eslint.config.cjs").to_str().unwrap(),
                    "--cwd",
                    project.0.to_str().unwrap(),
                    "--format",
                    "json",
                ])
                .output()
                .unwrap();
            assert_eq!(verified.status.code(), Some(3));
            let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
            assert_eq!(verified["observation"], "still_present", "{verified}");
            assert_eq!(verified["event_persisted"], true, "{verified}");
        }
        let status = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["status", project.0.to_str().unwrap(), "--format", "json"])
            .output()
            .unwrap();
        assert_eq!(
            status.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&status.stdout)
        );
        let status: Value = serde_json::from_slice(&status.stdout).unwrap();
        assert_eq!(status["open_task_count"], 1);
        assert_eq!(status["finding_count"], 1);
        assert_eq!(status["delivery_decision"], "not_evaluated");
        let shown = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "task",
                "show",
                fact["id"].as_str().unwrap(),
                project.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(shown.status.code(), Some(0));
        let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
        assert_eq!(shown["task"]["checker_id"], "node.eslint");
        assert_eq!(shown["state"], "open");
        assert_eq!(shown["delivery_decision"], "not_evaluated");
        let current = project
            .0
            .join("codeguard/tasks")
            .join(format!("{}.md", fact["id"].as_str().unwrap()));
        let text = std::fs::read_to_string(&current).unwrap();
        for required in [
            "问题证据",
            "规则依据",
            "允许范围",
            "修复步骤",
            "复检",
            "历史尝试",
            "关闭条件",
        ] {
            assert!(text.contains(required));
        }
        if round == 0 {
            std::fs::write(&current, format!("{text}\n用户备注：[x] 已修复\n")).unwrap();
            task_path = Some(current);
        } else {
            assert!(text.contains("用户备注"));
            assert_eq!(Some(current), task_path);
        }
    }
    let config = project.0.join("eslint.config.cjs");
    let original = std::fs::read(&config).unwrap();
    std::fs::write(&config, "module.exports=[]").unwrap();
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let brief: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        brief["repair_brief"]["disposition"],
        "verification_required"
    );
    assert_eq!(brief["repair_brief"]["recheck_argv"][2], "typescript");
    std::fs::write(&config, original).unwrap();
    let first = std::fs::read_dir(project.0.join("codeguard/reports"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut forged: Value = serde_json::from_slice(&std::fs::read(first).unwrap()).unwrap();
    forged["run_id"] = serde_json::json!("eslint-0-999999");
    forged["delivery_decision"] = serde_json::json!("allow");
    std::fs::write(
        project.0.join("codeguard/reports/eslint-0-999999.json"),
        serde_json::to_vec(&forged).unwrap(),
    )
    .unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "work",
            "sync",
            project.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let summary: Value = serde_json::from_slice(&sync.stdout).unwrap();
    assert_eq!(summary["failed_reports"], 1);
    assert_eq!(summary["new_findings"], 0);
    assert_eq!(
        std::fs::read_dir(project.0.join("codeguard/findings"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn eslint_missing_context_creates_one_environment_task_and_never_source_finding() {
    let project = project();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    for round in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                project.0.join("app.js").to_str().unwrap(),
                "--workspace",
                project.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(feedback["workbench_status"], "synced_partial", "{feedback}");
        assert_eq!(
            feedback["workbench"]["new_blockers"],
            if round == 0 { 1 } else { 0 }
        );
        assert_eq!(feedback["workbench"]["new_findings"], 0);
        let brief = &feedback["workbench"]["next"]["repair_brief"];
        assert_eq!(brief["kind"], "blocker");
        assert_eq!(brief["checker_id"], "node.eslint.preparation");
        assert_eq!(brief["recheck_argv"][2], "typescript");
        assert!(
            feedback["next_action"]
                .as_str()
                .unwrap()
                .contains("不修改无关源码")
        );
        assert!(brief["step"].as_str().unwrap().contains("不修改无关源码"));
    }
    let dirs: Vec<_> = std::fs::read_dir(project.0.join("codeguard/findings"))
        .unwrap()
        .collect();
    assert_eq!(dirs.len(), 1);
    let fact: Value = serde_json::from_slice(
        &std::fs::read(dirs[0].as_ref().unwrap().path().join("finding.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(fact["kind"], "blocker");
    assert_eq!(fact["state"], "open");
    let unavailable = lint(
        &project,
        &project.0.join("missing-node"),
        &project.0.join("missing-entry"),
    );
    let unavailable: Value = serde_json::from_slice(&unavailable.stdout).unwrap();
    assert_eq!(unavailable["reason"], "eslint_input_unavailable");
    assert_eq!(unavailable["workbench"]["new_blockers"], 0, "{unavailable}");
    assert_eq!(
        unavailable["workbench"]["next"]["repair_brief"]["preparation_guidance"]["diagnostic_reason"],
        "eslint_input_unavailable"
    );
    use std::os::unix::fs::PermissionsExt;
    let node = project.0.join("node");
    let entry = project.0.join("eslint.cjs");
    std::fs::write(&entry, "fixture").unwrap();
    let native = serde_json::json!([{"filePath":project.0.join("app.js"),"messages":[],"suppressedMessages":[],"errorCount":0,"warningCount":0,"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
    std::fs::write(&node,format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\nprintf '%s' '{}' > \"$report\"\n",native)).unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let recovered = lint(&project, &node, &entry);
    let recovered: Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(
        recovered["workbench_status"], "synced_partial",
        "{recovered}"
    );
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["checker_id"],
        "node.eslint.preparation"
    );
    assert_eq!(
        next["repair_brief"]["disposition"], "verification_required",
        "{next}"
    );
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("不再沿用旧环境诊断")
    );
    assert_eq!(
        next["repair_brief"]["recheck_argv"][3],
        project.0.join("app.js").to_str().unwrap()
    );
}

#[test]
fn eslint_external_preparation_target_cannot_enter_workspace_tasks() {
    let workspace = project();
    let external = project();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            workspace.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let result = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            external.0.join("app.js").to_str().unwrap(),
            "--workspace",
            workspace.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["workbench_status"], "source_outside_workspace");
    assert_eq!(
        std::fs::read_dir(workspace.0.join("codeguard/findings"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0; native invalid rule configuration"]
fn native_eslint_configuration_failure_becomes_preparation_not_source_violation() {
    let project = project();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    std::fs::write(
        project.0.join("eslint.config.cjs"),
        "module.exports=[{rules:{'codeguard-rule-that-does-not-exist':'error'}}]",
    )
    .unwrap();
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    let output = lint(
        &project,
        std::path::Path::new(&node),
        std::path::Path::new(&entry),
    );
    assert_eq!(output.status.code(), Some(3));
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["local_coherent"], false, "{feedback}");
    assert_eq!(feedback["workbench_status"], "synced_partial", "{feedback}");
    assert_eq!(feedback["workbench"]["new_findings"], 0);
    assert_eq!(feedback["workbench"]["new_blockers"], 1);
    assert_eq!(
        feedback["workbench"]["next"]["repair_brief"]["checker_id"],
        "node.eslint.preparation"
    );
    assert_eq!(feedback["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0; imported config disables original rule"]
fn native_imported_rule_disable_never_looks_like_source_repair() {
    let project = project();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    std::fs::write(project.0.join("mode.cjs"), "module.exports='error';").unwrap();
    std::fs::write(
        project.0.join("eslint.config.cjs"),
        "module.exports=[{rules:{'no-debugger':require('./mode.cjs')}}];",
    )
    .unwrap();
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    let first = lint(
        &project,
        std::path::Path::new(&node),
        std::path::Path::new(&entry),
    );
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["findings"].as_array().unwrap().len(), 1, "{first}");
    let id = first["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    std::fs::write(project.0.join("mode.cjs"), "module.exports='off';").unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            id,
            project.0.to_str().unwrap(),
            "--node-tool",
            std::path::Path::new(&node).to_str().unwrap(),
            "--eslint-entry",
            std::path::Path::new(&entry).to_str().unwrap(),
            "--eslint-version",
            "10.11.0",
            "--config",
            project.0.join("eslint.config.cjs").to_str().unwrap(),
            "--cwd",
            project.0.to_str().unwrap(),
            "--timeout",
            "90s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(
        verified["observation"], "rule_coverage_requires_review",
        "{verified}"
    );
    assert_eq!(verified["native_scan"]["original_context_matches"], true);
    assert_eq!(
        verified["native_scan"]["effective_rule"]["status"],
        "observed"
    );
    assert_eq!(verified["native_scan"]["effective_rule"]["severity"], 0);
    assert_eq!(verified["event_persisted"], true);
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["disposition"], "verification_required",
        "{next}"
    );
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("未启用原规则"),
        "{next}"
    );
    assert_eq!(
        std::fs::read_to_string(project.0.join("app.js")).unwrap(),
        "debugger;"
    );
}

#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0; real project-local plugin rule"]
fn native_project_local_plugin_retains_rule_and_rechecks_same_configuration() {
    let project = project();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let plugin = project.0.join("guard.cjs");
    let plugin_body = "module.exports={rules:{'no-debugger':{meta:{type:'problem',schema:[]},create(context){return {DebuggerStatement(node){context.report({node,message:'local guard rule'});}};}}}};";
    std::fs::write(&plugin, plugin_body).unwrap();
    let config_body = "module.exports=[{plugins:{workspace:require('./guard.cjs')},rules:{'workspace/no-debugger':'error'}}];";
    std::fs::write(project.0.join("eslint.config.cjs"), config_body).unwrap();
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    let first = lint(
        &project,
        std::path::Path::new(&node),
        std::path::Path::new(&entry),
    );
    assert_eq!(first.status.code(), Some(3));
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        first["findings"][0]["rule_id"], "workspace/no-debugger",
        "{first}"
    );
    let task = first["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    std::fs::write(project.0.join("app.js"), "console.log('repaired');").unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task,
            project.0.to_str().unwrap(),
            "--node-tool",
            std::path::Path::new(&node).to_str().unwrap(),
            "--eslint-entry",
            std::path::Path::new(&entry).to_str().unwrap(),
            "--eslint-version",
            "10.11.0",
            "--config",
            project.0.join("eslint.config.cjs").to_str().unwrap(),
            "--cwd",
            project.0.to_str().unwrap(),
            "--timeout",
            "120s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(verified.status.code(), Some(3));
    let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(
        verified["observation"], "candidate_absent_unverified_policy",
        "{verified}"
    );
    assert_eq!(verified["native_scan"]["effective_rule"]["severity"], 2);
    assert_eq!(
        verified["native_scan"]["effective_rule"]["rule_id"],
        "workspace/no-debugger"
    );
    assert_eq!(verified["event_persisted"], true);
    assert_eq!(verified["delivery_decision"], "not_evaluated");
    assert_eq!(std::fs::read_to_string(plugin).unwrap(), plugin_body);
    assert_eq!(
        std::fs::read_to_string(project.0.join("eslint.config.cjs")).unwrap(),
        config_body
    );
    let status = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["status", project.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["delivery_decision"], "not_evaluated");
}
