#![cfg(unix)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("codeguard-python-cve-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn native_tool(&self) -> PathBuf {
        self.native_tool_script("#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[{\"name\":\"flask\",\"version\":\"0.5\",\"vulns\":[{\"id\":\"PYSEC-2019-179\",\"fix_versions\":[\"1.0\"],\"aliases\":[\"CVE-2019-1010083\"]}]}],\"fixes\":[]}'\nexit 1\n")
    }

    fn native_tool_script(&self, script: &str) -> PathBuf {
        let tool = self.0.join("native-pip-audit");
        fs::write(&tool, script).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }

    fn check(&self, tool: &Path) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "python",
                self.0.to_str().unwrap(),
                "--pip-audit-tool",
                tool.to_str().unwrap(),
                "--pip-audit-version",
                "2.10.0",
                "--format=json",
            ])
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "invalid CLI JSON: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.code().unwrap(), report)
    }

    fn check_with_timeout(&self, tool: &Path, timeout: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "python",
                self.0.to_str().unwrap(),
                "--pip-audit-tool",
                tool.to_str().unwrap(),
                "--pip-audit-version",
                "2.10.0",
                "--timeout",
                timeout,
                "--format=json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }

    fn check_all(&self, tool: Option<&Path>) -> (i32, Value) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args(["check", "all", self.0.to_str().unwrap(), "--format=json"]);
        if let Some(tool) = tool {
            command.args([
                "--pip-audit-tool",
                tool.to_str().unwrap(),
                "--pip-audit-version",
                "2.10.0",
            ]);
        }
        let output = command.output().unwrap();
        let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "invalid check JSON: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.code().unwrap(), report)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn native_python_cve_result_is_visible_but_never_a_delivery_pass() {
    let project = Project::new();
    let manifest = b"[project]\nname = 'sample'\nversion = '0.1.0'\n";
    let lock = b"lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'flask'\nversion = '0.5'\n";
    fs::write(project.0.join("pyproject.toml"), manifest).unwrap();
    fs::write(project.0.join("pylock.toml"), lock).unwrap();
    let tool = project.native_tool();
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "python_cve_local_observation");
    assert_eq!(report["reason"], "native_advisories_observed_unverified");
    assert_eq!(report["findings"][0]["advisory_id"], "PYSEC-2019-179");
    assert_eq!(report["findings"][0]["package_version"], "0.5");
    assert_eq!(report["advisory_coverage"], "not_evaluated");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/python-cve-local-observation.schema.json"
    ))
    .unwrap();
    let required: BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field.as_str().unwrap())
        .collect();
    let actual: BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(required, actual);
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        fs::read(project.0.join("pyproject.toml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read(project.0.join("pylock.toml")).unwrap(), lock);
}

#[test]
fn valid_python_advisory_after_native_failure_remains_partial_evidence() {
    let project = Project::new();
    fs::write(project.0.join("main.py"), "print('sample')\n").unwrap();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("pylock.toml"),
        "lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'flask'\nversion = '0.5'\n",
    )
    .unwrap();
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
    let tool = project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[{\"name\":\"flask\",\"version\":\"0.5\",\"vulns\":[{\"id\":\"PYSEC-2019-179\",\"fix_versions\":[\"1.0\"],\"aliases\":[\"CVE-2019-1010083\"]}]}],\"fixes\":[]}'\nexit 2\n",
    );
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "pip_audit_execution_incomplete");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["native_report_valid"], true);
    assert_eq!(report["command_status"], "native_observed");
    assert_eq!(report["native_exit_code"], 2);
    assert_eq!(report["findings"][0]["advisory_id"], "PYSEC-2019-179");
    assert!(
        report["next_action"]
            .as_str()
            .is_some_and(|value| value.contains("已同步稳定 Python CVE 待处理任务"))
    );
    let tasks: Vec<_> = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            fs::read_to_string(entry.path())
                .is_ok_and(|body| body.contains("Python CVE 检查待处理"))
        })
        .collect();
    assert_eq!(tasks.len(), 1);

    let (all_exit, all) = project.check_all(Some(&tool));
    assert_eq!(all_exit, 3);
    assert_eq!(
        all["native_results"]["python_cve"][0]["reason"],
        "pip_audit_execution_incomplete"
    );
    assert_eq!(
        all["native_results"]["python_cve"][0]["findings"][0]["advisory_id"],
        "PYSEC-2019-179"
    );
    assert!(
        all["execution_tasks"]
            .as_array()
            .is_some_and(|tasks| tasks.iter().any(|task| task["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("python.cve."))
                && task["status"] == "native_incomplete"))
    );
    let cve = all["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["language"] == "python" && candidate["category"] == "cve")
        .unwrap();
    assert_eq!(cve["status"], "native_incomplete");
    assert!(
        all["unresolved_conditions"]
            .as_array()
            .is_some_and(|conditions| conditions
                .iter()
                .any(|condition| condition == "python_cve_task_incomplete"))
    );

    project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":'\nexit 2\n",
    );
    let (broken_exit, broken) = project.check(&tool);
    assert_eq!(broken_exit, 3);
    assert_eq!(broken["local_scan_complete"], false);
    assert_eq!(broken["findings"], serde_json::json!([]));

    project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[{\"name\":\"jinja2\",\"version\":\"3.0.2\",\"vulns\":[{\"id\":\"PYSEC-2021-1\",\"fix_versions\":[],\"aliases\":[]}]}],\"fixes\":[]}'\nexit 2\n",
    );
    let (foreign_exit, foreign) = project.check(&tool);
    assert_eq!(foreign_exit, 3);
    assert_eq!(foreign["reason"], "pip_audit_lock_attribution_unverified");
    assert_eq!(foreign["findings"], serde_json::json!([]));

    project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[{\"name\":\"flask\",\"version\":\"0.5\",\"vulns\":[{\"id\":\"PYSEC-2019-179\",\"fix_versions\":[\"1.0\"],\"aliases\":[\"CVE-2019-1010083\"]}]}],\"fixes\":[]}'\n/bin/sleep 5\n",
    );
    let (timeout_exit, timeout_report) = project.check_with_timeout(&tool, "2s");
    assert_eq!(timeout_exit, 3);
    assert_eq!(timeout_report["reason"], "request_deadline_exceeded");
    assert_eq!(timeout_report["local_scan_complete"], false);
    assert_eq!(timeout_report["native_report_valid"], true);
    assert_eq!(timeout_report["native_exit_code"], serde_json::Value::Null);
    assert_eq!(
        timeout_report["findings"][0]["advisory_id"], "PYSEC-2019-179",
        "{timeout_report}"
    );
    assert!(
        timeout_report["next_action"]
            .as_str()
            .is_some_and(|value| value.contains("已同步稳定 Python CVE 待处理任务"))
    );

    project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[{\"name\":\"flask\",\"version\":\"0.5\",\"vulns\":[{\"id\":\"PYSEC-2019-179\",\"fix_versions\":[\"1.0\"],\"aliases\":[\"CVE-2019-1010083\"]}]}],\"fixes\":[]}'\n/bin/sleep 1\n/bin/dd if=/dev/zero bs=1048576 count=17 1>&2 2>/dev/null\n",
    );
    let (limited_exit, limited_report) = project.check(&tool);
    assert_eq!(limited_exit, 3);
    assert_eq!(limited_report["reason"], "pip_audit_output_limit");
    assert_eq!(limited_report["native_report_valid"], true);
    assert_eq!(limited_report["native_exit_code"], serde_json::Value::Null);
    assert_eq!(limited_report["local_scan_complete"], false);
    assert_eq!(
        limited_report["findings"][0]["advisory_id"],
        "PYSEC-2019-179"
    );
    let (all_limited_exit, all_limited) = project.check_all(Some(&tool));
    assert_eq!(all_limited_exit, 3);
    assert_eq!(
        all_limited["native_results"]["python_cve"][0]["reason"],
        "pip_audit_output_limit"
    );
    assert_eq!(
        all_limited["native_results"]["python_cve"][0]["findings"][0]["advisory_id"],
        "PYSEC-2019-179"
    );
    assert!(
        all_limited["execution_tasks"]
            .as_array()
            .is_some_and(|tasks| tasks.iter().any(|task| task["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("python.cve."))
                && task["status"] == "native_incomplete"))
    );

    project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":'\n/bin/sleep 1\n/bin/dd if=/dev/zero bs=1048576 count=17 1>&2 2>/dev/null\n",
    );
    let (truncated_exit, truncated_report) = project.check(&tool);
    assert_eq!(truncated_exit, 3);
    assert_eq!(truncated_report["reason"], "pip_audit_output_limit");
    assert_eq!(truncated_report["native_report_valid"], false);
    assert_eq!(truncated_report["findings"], serde_json::json!([]));
}

#[test]
fn missing_standard_lock_preserves_a_specific_blocker() {
    let project = Project::new();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\n",
    )
    .unwrap();
    let tool = project.native_tool();
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "standard_python_lock_missing");
    assert_eq!(report["findings"], serde_json::json!([]));
}

#[test]
fn conditional_pylock_runs_native_audit_without_attributing_unselected_packages() {
    let project = Project::new();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname='sample'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("pylock.toml"),
        "lock-version='1.0'\ncreated-by='fixture'\n[[packages]]\nname='flask'\nversion='0.5'\nmarker=\"sys_platform == 'win32'\"\n",
    )
    .unwrap();
    let tool = project.native_tool();
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "python_lock_selection_unresolved");
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["native_report_valid"], false);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["native_version"], "2.10.0");
    assert_eq!(report["native_exit_code"], 1);
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .contains("目标 Python 环境")
    );
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .contains("1 个待归属 advisory")
    );
    let (check_exit, all) = project.check_all(Some(&tool));
    assert_eq!(check_exit, 3);
    assert_eq!(
        all["native_results"]["python_cve"][0]["reason"],
        "python_lock_selection_unresolved"
    );
    assert_eq!(
        all["native_results"]["python_cve"][0]["findings"],
        serde_json::json!([])
    );
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
    let (initialized_exit, initialized) = project.check_all(Some(&tool));
    assert_eq!(initialized_exit, 3);
    assert_eq!(
        initialized["native_results"]["python_cve"][0]["backlog_status"],
        "synced_partial"
    );
    let tasks: Vec<_> = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            fs::read_to_string(entry.path())
                .is_ok_and(|body| body.contains("Python CVE 检查待处理"))
        })
        .collect();
    assert_eq!(tasks.len(), 1);
}

#[test]
fn native_zero_advisories_keeps_database_and_delivery_unverified() {
    let project = Project::new();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("pylock.toml"),
        "lock-version = '1.0'\ncreated-by = 'fixture'\npackages = []\n",
    )
    .unwrap();
    let tool = project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[],\"fixes\":[]}'\nexit 0\n",
    );
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "native_zero_advisories_unverified");
    assert_eq!(report["native_report_valid"], true);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["advisory_coverage"], "not_evaluated");
}

#[test]
fn mutation_of_private_lock_invalidates_a_native_advisory_result() {
    let project = Project::new();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\n",
    )
    .unwrap();
    let lock = b"lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'flask'\nversion = '0.5'\n";
    fs::write(project.0.join("pylock.toml"), lock).unwrap();
    let tool = project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\necho '# changed' >> \"$2/pylock.toml\"\nprintf '%s' '{\"dependencies\":[{\"name\":\"flask\",\"version\":\"0.5\",\"vulns\":[{\"id\":\"PYSEC-2019-179\",\"fix_versions\":[\"1.0\"],\"aliases\":[]}]}],\"fixes\":[]}'\nexit 1\n",
    );
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "python_cve_input_changed");
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(fs::read(project.0.join("pylock.toml")).unwrap(), lock);
}

#[test]
fn advisory_for_package_absent_from_standard_lock_is_not_a_project_finding() {
    let project = Project::new();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\n",
    )
    .unwrap();
    fs::write(project.0.join("pylock.toml"), "lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'jinja2'\nversion = '3.0.2'\n").unwrap();
    let tool = project.native_tool();
    let (exit, report) = project.check(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["reason"], "pip_audit_lock_attribution_unverified");
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["native_report_valid"], false);
}

#[test]
fn check_all_observes_each_python_build_root_without_allowing_delivery() {
    let project = Project::new();
    for build in [project.0.clone(), project.0.join("service")] {
        fs::create_dir_all(&build).unwrap();
        fs::write(build.join("main.py"), "print('sample')\n").unwrap();
        fs::write(
            build.join("pyproject.toml"),
            "[project]\nname = 'sample'\nversion = '0.1.0'\n",
        )
        .unwrap();
        fs::write(build.join("pylock.toml"), "lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'flask'\nversion = '0.5'\n").unwrap();
    }
    let tool = project.native_tool();
    let (exit, report) = project.check_all(Some(&tool));
    assert_eq!(exit, 3);
    assert_eq!(report["schema_version"], "0.31.0");
    assert_eq!(report["delivery_decision"], "incomplete");
    let scans = report["native_results"]["python_cve"].as_array().unwrap();
    assert_eq!(scans.len(), 2);
    assert_eq!(scans[0]["build_root"], ".");
    assert_eq!(scans[1]["build_root"], "service");
    for scan in scans {
        assert_eq!(scan["reason"], "native_advisories_observed_unverified");
        assert_eq!(scan["findings"][0]["advisory_id"], "PYSEC-2019-179");
        assert_eq!(scan["coverage_proven"], false);
    }
    let cve = report["category_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["language"] == "python" && candidate["category"] == "cve")
        .unwrap();
    assert_eq!(cve["status"], "observed_unverified");
}

#[test]
fn check_all_missing_native_context_reports_a_blocker_without_findings() {
    let project = Project::new();
    fs::write(project.0.join("main.py"), "print('sample')\n").unwrap();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        project.0.join("pylock.toml"),
        "lock-version = '1.0'\ncreated-by = 'fixture'\npackages = []\n",
    )
    .unwrap();
    let (exit, report) = project.check_all(None);
    assert_eq!(exit, 3);
    let scan = &report["native_results"]["python_cve"][0];
    assert_eq!(scan["reason"], "pip_audit_native_context_missing");
    assert_eq!(scan["findings"], serde_json::json!([]));
    assert_eq!(scan["native_report_valid"], false);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn initialized_python_cve_creates_one_stable_task_for_repeated_scans() {
    let project = Project::new();
    fs::write(project.0.join("main.py"), "print('sample')\n").unwrap();
    fs::write(
        project.0.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(project.0.join("pylock.toml"), "lock-version = '1.0'\ncreated-by = 'fixture'\n[[packages]]\nname = 'flask'\nversion = '0.5'\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        init.status.code(),
        Some(3),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let tool = project.native_tool();
    for _ in 0..2 {
        let (exit, report) = project.check_all(Some(&tool));
        assert_eq!(exit, 3);
        assert_eq!(
            report["native_results"]["python_cve"][0]["backlog_status"], "synced_partial",
            "{report}"
        );
    }
    let tasks: Vec<_> = fs::read_dir(project.0.join(".codeguard/tasks"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".md"))
        .filter(|entry| {
            fs::read_to_string(entry.path())
                .is_ok_and(|body| body.contains("Python CVE 检查待处理"))
        })
        .collect();
    assert_eq!(tasks.len(), 1);
    let id = tasks[0]
        .path()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let shown = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "show",
            &id,
            project.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&shown.stdout),
        String::from_utf8_lossy(&shown.stderr)
    );
    let view: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(view["task"]["checker_id"], "python.pip_audit");
    let before = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(
        before.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&before.stdout)
    );
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            project.0.to_str().unwrap(),
            "--pip-audit-tool",
            tool.to_str().unwrap(),
            "--pip-audit-version",
            "2.10.0",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(verify.status.code(), Some(3));
    let result: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(result["observation"], "still_blocked", "{result}");
    assert_eq!(result["event_persisted"], true, "{result}");
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
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", project.0.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(
        next.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&next.stdout)
    );
    fs::write(
        project.0.join("pylock.toml"),
        "lock-version = '1.0'\ncreated-by = 'fixture'\npackages = []\n",
    )
    .unwrap();
    let zero_tool = project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[],\"fixes\":[]}'\nexit 0\n",
    );
    let zero = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &id,
            project.0.to_str().unwrap(),
            "--pip-audit-tool",
            zero_tool.to_str().unwrap(),
            "--pip-audit-version",
            "2.10.0",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(zero.status.code(), Some(3));
    let zero_report: Value = serde_json::from_slice(&zero.stdout).unwrap();
    assert_eq!(
        zero_report["native_scan"]["scan"]["reason"], "native_zero_advisories_unverified",
        "{zero_report}"
    );
    assert_eq!(zero_report["observation"], "still_blocked");
    assert_eq!(zero_report["event_persisted"], true);
    let after: Value = serde_json::from_slice(
        &fs::read(
            project
                .0
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(after["state"], "open");
}

#[test]
fn initialized_multi_root_python_cve_keeps_distinct_stable_tasks() {
    let project = Project::new();
    for build in [project.0.clone(), project.0.join("service")] {
        fs::create_dir_all(&build).unwrap();
        fs::write(build.join("main.py"), "print('sample')\n").unwrap();
        fs::write(
            build.join("pyproject.toml"),
            "[project]\nname = 'sample'\nversion = '0.1.0'\n",
        )
        .unwrap();
        fs::write(
            build.join("pylock.toml"),
            "lock-version = '1.0'\ncreated-by = 'fixture'\npackages = []\n",
        )
        .unwrap();
    }
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
    let tool = project.native_tool_script(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'pip-audit 2.10.0'; exit 0; fi\nprintf '%s' '{\"dependencies\":[],\"fixes\":[]}'\nexit 0\n",
    );
    for _ in 0..2 {
        let (_, report) = project.check_all(Some(&tool));
        let scans = report["native_results"]["python_cve"].as_array().unwrap();
        assert_eq!(scans.len(), 2);
        assert!(
            scans
                .iter()
                .all(|scan| scan["backlog_status"] == "synced_partial")
        );
    }
    let facts: Vec<Value> = fs::read_dir(project.0.join(".codeguard/findings"))
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read(entry.path().join("finding.json")).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .filter(|fact: &Value| fact["checker_id"] == "python.pip_audit")
        .collect();
    assert_eq!(facts.len(), 2);
    let roots: BTreeSet<_> = facts
        .iter()
        .filter_map(|fact| fact["build_root"].as_str())
        .collect();
    assert_eq!(roots, BTreeSet::from([".", "service"]));
    assert!(facts.iter().all(|fact| fact["state"] == "open"));
}
