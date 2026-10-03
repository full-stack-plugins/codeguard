#![cfg(unix)]

use codeguard_adapters::bundled_ruff_rulepack;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("codeguard-lint-cli-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run(project: &Project, extra: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .args(extra)
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .expect("run CLI");
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = serde_json::from_slice(&output.stdout).expect("single JSON response");
    (output.status.code().unwrap(), document)
}

#[test]
fn selected_file_feedback_never_claims_full_python_coverage() {
    let project = Project::new();
    fs::write(project.0.join("changed.py"), "import os\n").unwrap();
    fs::write(project.0.join("untouched.py"), "import sys\n").unwrap();
    let (exit, report) = run(&project, &["--file", "changed.py"]);
    assert_eq!(exit, 3);
    assert_eq!(report["scan_scope"], "selected_files");
    assert_eq!(report["requested_paths"], serde_json::json!(["changed.py"]));
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["path"], "changed.py");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["backlog_status"], "not_synced_scoped");
}

#[test]
fn unknown_selected_file_is_incomplete_and_does_not_scan_another_file() {
    let project = Project::new();
    fs::write(project.0.join("untouched.py"), "import sys\n").unwrap();
    let (exit, report) = run(&project, &["--file", "missing.py"]);
    assert_eq!(exit, 3);
    assert_eq!(report["scan_scope"], "selected_files");
    assert!(report["files"].as_array().unwrap().is_empty());
    assert_eq!(report["local_scan_complete"], false);
    assert!(
        report["incomplete_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "target_not_discovered:missing.py")
    );
}

#[test]
fn selected_paths_reject_escape_and_over_budget_before_scanning() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    for file in ["../outside.py", "/tmp/outside.py", "dir\\outside.py"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "python",
                project.0.to_str().unwrap(),
                "--file",
                file,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{file}");
        assert!(output.stdout.is_empty(), "{file}");
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command.args(["lint", "python", project.0.to_str().unwrap()]);
    for index in 0..9 {
        command.args(["--file", &format!("file{index}.py")]);
    }
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn unrelated_nested_ruff_configuration_does_not_start_tool_for_selected_file() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    fs::write(project.0.join("changed.py"), "import os\n").unwrap();
    fs::create_dir(project.0.join("nested")).unwrap();
    fs::write(project.0.join("nested/untouched.py"), "import sys\n").unwrap();
    fs::write(
        project.0.join("nested/ruff.toml"),
        "[lint]\nselect = ['F']\n",
    )
    .unwrap();
    let marker = project.0.join("tool-was-started");
    let tool = project.0.join("fake-ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, report) = run(
        &project,
        &[
            "--file",
            "changed.py",
            "--ruff-tool",
            tool.to_str().unwrap(),
        ],
    );
    assert_eq!(exit, 3);
    assert!(!marker.exists());
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["path"], "changed.py");
    assert_eq!(report["files"][0]["run_status"], "incomplete");
}

#[test]
#[ignore = "requires native Ruff 0.16.8 via CODEGUARD_RUFF_BIN"]
fn selected_file_runs_native_ruff_without_importing_partial_backlog() {
    let project = Project::new();
    fs::write(project.0.join("changed.py"), "import os\n").unwrap();
    fs::write(project.0.join("untouched.py"), "import sys\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let reports_dir = project.0.join(".codeguard/reports");
    let before_reports = fs::read_dir(&reports_dir).unwrap().count();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, report) = run(&project, &["--file", "changed.py", "--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["path"], "changed.py");
    assert_eq!(report["files"][0]["run_status"], "findings");
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    assert!(report.get("syntax_precheck").is_none());
    assert_eq!(report["backlog_status"], "not_synced_scoped");
    assert_eq!(fs::read_dir(reports_dir).unwrap().count(), before_reports);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn sigint_during_native_probe_reaps_descendants_and_returns_130() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    use std::thread;

    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let ready = project.0.join("native-ready");
    let late = project.0.join("native-late-write");
    let tool = project.0.join("ruff");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\n(sleep 0.6; /usr/bin/touch '{}') &\n/usr/bin/touch '{}'\nwait\n",
            late.display(),
            ready.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--timeout",
            "2s",
            "--format=json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(2) {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(ready.exists(), "原生进程未开始，不能验证运行中取消");
    let interrupt = Command::new("/bin/kill")
        .args(["-INT", &child.id().to_string()])
        .output()
        .unwrap();
    assert!(interrupt.status.success());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["exit_code"], 130);
    assert!(
        report["incomplete_reasons"]
            .to_string()
            .contains("request_cancelled")
    );
    thread::sleep(Duration::from_millis(650));
    assert!(!late.exists(), "Ctrl-C 返回后原生子孙进程仍写入文件");
}

#[test]
fn lint_python_rejects_invalid_budget_before_native_probe() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let marker = project.0.join("native-started");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for budget in ["0ms", "-1s", "25h", "unknown"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "python",
                project.0.to_str().unwrap(),
                "--timeout",
                budget,
                "--ruff-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{budget}");
        assert!(output.stdout.is_empty());
        assert!(!marker.exists());
    }
    let invalid_environment = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("CODEGUARD_TIMEOUT", "unlimited")
        .output()
        .unwrap();
    assert_eq!(invalid_environment.status.code(), Some(2));
    assert!(invalid_environment.stdout.is_empty());
    assert!(!marker.exists());
    let override_environment = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--timeout",
            "100ms",
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env("CODEGUARD_TIMEOUT", "unlimited")
        .output()
        .unwrap();
    assert_eq!(override_environment.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&override_environment.stdout).unwrap();
    assert_eq!(report["execution_budget"]["source"], "cli");
}

#[test]
fn lint_python_budget_expires_during_native_version_probe() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = project.0.join("ruff");
    fs::write(&tool, "#!/bin/sh\nsleep 2\nprintf 'ruff 0.16.8\\n'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let started = Instant::now();
    let (exit, report) = run(
        &project,
        &["--timeout", "100ms", "--ruff-tool", tool.to_str().unwrap()],
    );
    assert_eq!(exit, 3);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(report["files"][0]["reason"], "request_deadline_exceeded");
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["execution_budget"]["source"], "cli");
    assert_eq!(
        report["execution_budget"]["enforcement"],
        "native_execution_only"
    );
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn unconfigured_project_returns_visible_missing_status_without_starting_tool() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", "/nonexistent/ruff"]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "python_lint_feedback");
    assert_eq!(
        report["checker_configurations"][0]["configuration"],
        "missing"
    );
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["workspace_binding"], "uninitialized");
    assert!(report["workspace_id"].is_null());
    assert_eq!(report["repair_brief_status"], "unavailable");
    assert!(report["next"].is_null());
    assert!(report["files"][0]["tool_sha256"].is_null());
    assert!(report["files"][0]["config_sha256"].is_null());
}

#[test]
fn lint_python_reads_versioned_project_runtime_default_without_changing_quality() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s"}"#,
    )
    .unwrap();
    let (exit, report) = run(&project, &[]);
    assert_eq!(exit, 3);
    assert_eq!(report["execution_budget"]["timeout_ms"], 2000);
    assert_eq!(report["execution_budget"]["source"], "project_default");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
}

#[test]
fn initialized_lint_report_binds_the_existing_workspace_identity() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let workspace: Value =
        serde_json::from_slice(&fs::read(project.0.join(".codeguard/workspace.json")).unwrap())
            .unwrap();
    let (_, report) = run(&project, &["--ruff-tool", "/nonexistent/ruff"]);
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["schema_version"], "0.15.0");
    #[cfg(not(feature = "wasm-precheck"))]
    assert_eq!(report["schema_version"], "0.13.0");
    assert_eq!(report["execution_budget"]["timeout_ms"], 1_800_000);
    assert_eq!(report["execution_budget"]["source"], "builtin_default");
    assert_eq!(report["workspace_binding"], "bound");
    assert_eq!(report["workspace_id"], workspace["workspace_id"]);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["backlog_status"], "synced_partial");
    assert_eq!(report["backlog_sync"]["imported_reports"], 1);
    assert_eq!(report["repair_brief_status"], "available");
    assert_eq!(report["next"]["disposition"], "needs_decision");
    assert_eq!(
        report["next"]["repair_brief"]["reason_code"],
        "project_ruff_config_not_found"
    );
    assert_eq!(report["next"]["delivery_decision"], "not_evaluated");
    let run_id = report["run_id"].as_str().unwrap();
    let saved: Value = serde_json::from_slice(
        &fs::read(project.0.join(format!(".codeguard/reports/{run_id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["run_id"], run_id);
    assert_eq!(saved["workspace_id"], workspace["workspace_id"]);
    assert_eq!(saved["schema_version"], "0.9.0");
    assert!(saved.get("next").is_none());
    assert!(
        project
            .0
            .join(format!(".codeguard/state/consumed/{run_id}.json"))
            .exists()
    );
}

#[test]
fn brief_read_failure_keeps_native_scan_and_synced_backlog_visible() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::create_dir(project.0.join(".codeguard/findings/invalid-record")).unwrap();
    let (exit, report) = run(&project, &[]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert_eq!(report["backlog_status"], "synced_partial");
    assert_eq!(report["backlog_sync"]["new_blockers"], 1);
    assert_eq!(report["repair_brief_status"], "unavailable");
    assert_eq!(report["repair_brief_reason"], "finding_id_invalid");
    assert!(report["next"].is_null());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn configured_project_with_missing_tool_returns_tool_error_in_feedback() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", "/nonexistent/ruff"]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["checker_configurations"][0]["configuration"],
        "configured"
    );
    assert_eq!(report["files"][0]["reason"], "ruff_tool_not_found");
    assert_eq!(report["files"][0]["run_status"], "incomplete");
    assert!(report["files"][0]["tool_sha256"].is_null());
    assert!(report["files"][0]["config_sha256"].is_null());
}

#[test]
fn non_ruff_program_cannot_masquerade_as_a_configured_checker() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", "/bin/echo"]);
    assert_eq!(exit, 3);
    assert_eq!(report["files"][0]["reason"], "ruff_version_unavailable");
    assert_eq!(report["tool_approval"], "unverified");
}

#[test]
fn human_output_shows_configuration_and_a_repair_next_step() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "pass\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--format",
            "human",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let rendered = String::from_utf8(output.stdout).unwrap();
    assert!(rendered.contains("配置 missing"));
    assert!(rendered.contains("下一步") || rendered.contains("建议"));
    assert!(rendered.contains("not_evaluated"));
}

#[test]
fn initialized_human_output_includes_actionable_brief_without_quality_allow() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let rendered = String::from_utf8(output.stdout).unwrap();
    assert!(rendered.contains("下一步：needs_decision"));
    assert!(rendered.contains("任务 \"CG-B-"));
    assert!(rendered.contains("复检"));
    assert!(rendered.contains("not_evaluated"));
    assert!(!rendered.contains("PASS"));
}

#[test]
fn unsupported_language_is_rejected_before_execution() {
    let project = Project::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "ruby", project.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
#[ignore = "requires native Ruff; run with CODEGUARD_RUFF_BIN"]
fn configured_project_emits_native_findings_to_cli_feedback() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", &tool]);
    assert_eq!(exit, 3); // 尚无批准策略和完整门禁，不签发通过。
    assert_eq!(report["files"][0]["run_status"], "findings");
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    assert_eq!(report["files"][0]["findings"][0]["line"], 1);
    assert_eq!(report["schema_version"], "0.13.0");
    assert_eq!(
        report["files"][0]["rule_settings"]["globally_enabled_mapped_rules"],
        serde_json::json!(["F401"])
    );
    assert_eq!(
        report["files"][0]["rule_settings"]["coverage_proven"],
        false
    );
    assert_eq!(
        report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"],
        0
    );
    assert_eq!(report["native_tool_version"], "ruff 0.16.8");
    assert_eq!(
        report["files"][0]["findings"][0]["codeguard_rule_id"],
        "python.ruff.F401"
    );
    assert_eq!(
        report["files"][0]["findings"][0]["rulepack_sha256"],
        bundled_ruff_rulepack().unwrap().sha256
    );
    assert_eq!(
        report["files"][0]["findings"][0]["rulepack_status"],
        "candidate_unapproved"
    );
    assert_eq!(
        report["files"][0]["tool_sha256"],
        format!("{:x}", Sha256::digest(fs::read(&tool).unwrap()))
    );
    assert_eq!(
        report["files"][0]["config_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(project.0.join("ruff.toml")).unwrap())
        )
    );
    assert_eq!(report["tool_approval"], "unverified");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn native_noqa_is_visible_as_suppression_without_creating_active_finding_task() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os  # noqa: F401\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["schema_version"], "0.13.0");
    assert_eq!(report["files"][0]["run_status"], "suppressed");
    assert!(
        report["files"][0]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report["files"][0]["suppression_audit"]["suppressed_diagnostic_count"],
        1
    );
    assert_eq!(
        report["files"][0]["suppression_audit"]["suppressed_rule_ids"],
        serde_json::json!(["F401"])
    );
    assert_eq!(report["backlog_status"], "synced_partial");
    assert_eq!(report["backlog_sync"]["new_findings"], 0);
    assert_eq!(
        fs::read_dir(project.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
#[ignore = "requires native Ruff; run with CODEGUARD_RUFF_BIN"]
fn configured_project_resolves_ruff_from_absolute_path_entry() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "python",
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .env("PATH", tool.parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
}

#[test]
#[ignore = "requires native Ruff; run with CODEGUARD_RUFF_BIN"]
fn initialized_native_finding_returns_next_brief_in_same_cli_response() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F']\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let (exit, report) = run(&project, &["--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    assert_eq!(report["backlog_status"], "synced_partial");
    assert_eq!(report["repair_brief_status"], "available");
    assert_eq!(report["next"]["repair_brief"]["native_rule_id"], "F401");
    assert_eq!(report["next"]["disposition"], "actionable");
    assert_eq!(report["next"]["delivery_decision"], "not_evaluated");
}

#[test]
fn unrelated_symlink_configuration_does_not_pollute_selected_discovery() {
    let project = Project::new();
    fs::write(project.0.join("changed.py"), "pass\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::create_dir(project.0.join("unrelated")).unwrap();
    fs::write(project.0.join("unrelated/untouched.py"), "pass\n").unwrap();
    std::os::unix::fs::symlink("../ruff.toml", project.0.join("unrelated/ruff.toml")).unwrap();
    let (exit, report) = run(&project, &["--file", "changed.py"]);
    assert_eq!(exit, 3);
    assert!(
        !report["incomplete_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "discovery_incomplete"),
        "{report}"
    );
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["checker_configurations"].as_array().unwrap().len(),
        1
    );
}
