#![cfg(unix)]

use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
/// 受控项目与工具入口；各测试只修改自己创建的临时目录。
struct Fixture {
    root: PathBuf,
    bin: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-ruff-local-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("project");
        let bin = base.join("bin");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir(&bin).unwrap();
        fs::write(root.join("app.py"), "import os\n").unwrap();
        fs::write(root.join("ruff.toml"), "[lint]\nselect=['F401']\n").unwrap();
        Self { root, bin }
    }
    fn tool(&self, directory: &Path, bad_version: bool) -> PathBuf {
        fs::create_dir_all(directory).unwrap();
        let tool = directory.join("ruff");
        let marker = directory.join("started");
        let version = if bad_version {
            "echo wrong-tool; exit 2"
        } else {
            "echo 'ruff 0.16.8'; exit 0"
        };
        let script = r#"#!/bin/sh
printf '%s\n' "$1" >> '__MARKER__'
if [ "$1" = --version ]; then __VERSION__; fi
if [ "$2" = --show-files ]; then printf '%s\n' "$3"; exit 0; fi
if [ "$2" = --show-settings ]; then printf 'linter.rules.enabled = [\n\tunused-import (F401),\n]\nlinter.per_file_ignores = {}\n'; exit 0; fi
if [ "$2" = --no-cache ]; then
    if [ "$3" = --ignore-noqa ]; then source=$6; else source=$5; fi
    printf '[{"code":"F401","message":"unused","filename":"%s","location":{"row":1,"column":1},"severity":"error"}]\n' "$source"
    exit 1
fi
exit 2
"#;
        fs::write(
            &tool,
            script
                .replace("__MARKER__", marker.to_str().unwrap())
                .replace("__VERSION__", version),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn local_bin(&self) -> PathBuf {
        self.root.join(".venv/bin")
    }
    fn run(&self, args: &[&str], path: &str) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", path)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn lint(&self, extra: &[&str]) -> Value {
        let mut args = vec![
            "lint",
            "python",
            self.root.to_str().unwrap(),
            "--format=json",
        ];
        args.extend(extra);
        self.run(&args, self.bin.to_str().unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.root.parent().unwrap());
    }
}

#[test]
fn configured_root_uses_local_ruff_without_path_or_explicit_tool() {
    let f = Fixture::new();
    f.tool(&f.local_bin(), false);
    let report = f.lint(&[]);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    assert!(f.local_bin().join("started").exists());
}
#[test]
fn local_ruff_precedes_global_ruff_in_aggregate_checks() {
    let f = Fixture::new();
    f.tool(&f.local_bin(), false);
    f.tool(&f.bin, false);
    let report = f.run(
        &["check", "all", f.root.to_str().unwrap(), "--format=json"],
        f.bin.to_str().unwrap(),
    );
    assert_eq!(
        report["native_results"]["python_lint"]["local_scan_complete"], true,
        "{report}"
    );
    assert!(f.local_bin().join("started").exists());
    assert!(!f.bin.join("started").exists());
}
#[test]
fn local_ruff_reuses_one_task_and_rechecks_without_explicit_tool() {
    let f = Fixture::new();
    f.tool(&f.local_bin(), false);
    f.run(
        &["init", f.root.to_str().unwrap(), "--apply", "--format=json"],
        f.bin.to_str().unwrap(),
    );
    let report = f.lint(&[]);
    let id = report["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .expect("原生发现必须有稳定身份");
    let repeated = f.lint(&[]);
    assert_eq!(repeated["files"][0]["findings"][0]["finding_id"], id);
    assert_eq!(repeated["backlog_sync"]["new_findings"], 0);
    let verified = f.run(
        &[
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--format=json",
        ],
        f.bin.to_str().unwrap(),
    );
    assert_eq!(verified["observation"], "still_present", "{verified}");
    assert_eq!(verified["event_persisted"], true);
}
#[test]
fn explicit_bad_ruff_never_uses_local_or_global_tools() {
    let f = Fixture::new();
    f.tool(&f.local_bin(), false);
    f.tool(&f.bin, false);
    let report = f.lint(&["--ruff-tool", "/nonexistent/ruff"]);
    assert_eq!(report["files"][0]["reason"], "ruff_tool_not_found");
    assert!(!f.local_bin().join("started").exists());
    assert!(!f.bin.join("started").exists());
}
#[test]
fn local_version_failure_never_falls_back_to_global_ruff() {
    let f = Fixture::new();
    f.tool(&f.local_bin(), true);
    f.tool(&f.bin, false);
    let report = f.lint(&[]);
    assert_eq!(
        report["files"][0]["reason"], "ruff_version_unavailable",
        "{report}"
    );
    assert!(f.local_bin().join("started").exists());
    assert!(!f.bin.join("started").exists());
}
#[test]
fn damaged_local_entry_stays_a_blocker_without_global_fallback() {
    let f = Fixture::new();
    fs::create_dir_all(f.local_bin()).unwrap();
    fs::write(f.local_bin().join("ruff"), "not executable").unwrap();
    fs::set_permissions(
        f.local_bin().join("ruff"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    f.tool(&f.bin, false);
    let report = f.lint(&[]);
    assert_eq!(
        report["files"][0]["reason"], "ruff_local_tool_invalid",
        "{report}"
    );
    assert!(
        report["files"][0]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!f.bin.join("started").exists());
}
#[test]
fn local_directory_symlink_is_not_a_project_tool_context() {
    let f = Fixture::new();
    f.tool(&f.bin, false);
    symlink(&f.bin, f.root.join(".venv")).unwrap();
    let report = f.lint(&[]);
    assert_eq!(
        report["files"][0]["reason"], "ruff_local_tool_invalid",
        "{report}"
    );
    assert!(!f.bin.join("started").exists());
}
#[test]
fn missing_local_entry_keeps_existing_path_discovery() {
    let f = Fixture::new();
    fs::create_dir_all(f.local_bin()).unwrap();
    f.tool(&f.bin, false);
    let report = f.lint(&[]);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert!(f.bin.join("started").exists());
}
#[test]
fn unconfigured_root_does_not_start_local_or_global_ruff() {
    let f = Fixture::new();
    fs::remove_file(f.root.join("ruff.toml")).unwrap();
    f.tool(&f.local_bin(), false);
    f.tool(&f.bin, false);
    let report = f.lint(&[]);
    assert_eq!(
        report["files"][0]["reason"],
        "project_ruff_config_not_found"
    );
    assert!(!f.local_bin().join("started").exists());
    assert!(!f.bin.join("started").exists());
}

#[test]
fn explicit_good_tool_overrides_even_a_damaged_local_environment() {
    let f = Fixture::new();
    fs::write(f.root.join(".venv"), "damaged environment").unwrap();
    let tool = f.tool(&f.bin, false);
    let report = f.lint(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert!(f.bin.join("started").exists());
}

#[test]
fn ordinary_local_directories_can_hold_a_symlink_to_a_bound_ruff_binary() {
    let f = Fixture::new();
    let target = f.tool(&f.bin, false);
    fs::create_dir_all(f.local_bin()).unwrap();
    symlink(&target, f.local_bin().join("ruff")).unwrap();
    let report = f.lint(&[]);
    assert_eq!(report["local_scan_complete"], true, "{report}");
}

#[test]
fn local_bin_link_and_broken_executable_remain_environment_blockers() {
    for bin_link in [true, false] {
        let f = Fixture::new();
        f.tool(&f.bin, false);
        fs::create_dir(f.root.join(".venv")).unwrap();
        if bin_link {
            symlink(&f.bin, f.local_bin()).unwrap();
        } else {
            fs::create_dir(f.local_bin()).unwrap();
            symlink(f.root.join("missing_binary"), f.local_bin().join("ruff")).unwrap();
        }
        let report = f.lint(&[]);
        assert_eq!(
            report["files"][0]["reason"], "ruff_local_tool_invalid",
            "{report}"
        );
        assert!(!f.bin.join("started").exists());
    }
}

#[test]
fn confirmed_edit_uses_local_ruff_for_only_the_requested_file() {
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    f.tool(&f.local_bin(), false);
    fs::write(f.root.join("untouched.py"), "import sys\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "hook",
            "execute",
            f.root.to_str().unwrap(),
            "--format=json",
            "--timeout=30s",
        ])
        .env("PATH", f.bin.to_str().unwrap())
        .env_remove("CODEGUARD_TIMEOUT")
        .env_remove("CODEGUARD_JOBS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = serde_json::json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["app.py"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let files = report["local_feedback"]["python_lint"]["files"]
        .as_array()
        .expect("Hook须反馈原生检查");
    assert_eq!(files.len(), 1, "{report}");
    assert_eq!(files[0]["path"], "app.py");
    assert_eq!(files[0]["run_status"], "findings", "{report}");
    assert_eq!(files[0]["findings"][0]["rule_id"], "F401");
}

#[test]
#[ignore = "requires existing Ruff 0.16.8 via CODEGUARD_RUFF_BIN; temporary binary copy only"]
fn actual_local_ruff_detects_rechecks_repairs_and_retains_unverified_closure() {
    let f = Fixture::new();
    let installed = PathBuf::from(
        std::env::var_os("CODEGUARD_RUFF_BIN").expect("explicit existing Ruff required"),
    );
    fs::create_dir_all(f.local_bin()).unwrap();
    fs::copy(installed, f.local_bin().join("ruff")).unwrap();
    fs::set_permissions(
        f.local_bin().join("ruff"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    f.run(
        &["init", f.root.to_str().unwrap(), "--apply", "--format=json"],
        f.bin.to_str().unwrap(),
    );
    let report = f.lint(&[]);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert_eq!(report["native_tool_version"], "ruff 0.16.8");
    let id = report["files"][0]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    assert_eq!(report["files"][0]["findings"][0]["rule_id"], "F401");
    let repeated = f.lint(&[]);
    assert_eq!(repeated["files"][0]["findings"][0]["finding_id"], id);
    assert_eq!(repeated["backlog_sync"]["new_findings"], 0);
    let verify = || {
        f.run(
            &[
                "task",
                "verify",
                id,
                f.root.to_str().unwrap(),
                "--format=json",
            ],
            f.bin.to_str().unwrap(),
        )
    };
    assert_eq!(verify()["observation"], "still_present");
    fs::write(f.root.join("app.py"), "pass\n").unwrap();
    assert_eq!(
        verify()["observation"],
        "candidate_absent_unverified_policy"
    );
    fs::write(f.root.join("app.py"), "import os\n").unwrap();
    assert_eq!(verify()["observation"], "still_present");
}

#[test]
fn damaged_local_environment_has_one_specific_repair_task_and_original_recheck() {
    let f = Fixture::new();
    let tool = f.tool(&f.local_bin(), false);
    fs::set_permissions(tool, fs::Permissions::from_mode(0o600)).unwrap();
    f.tool(&f.bin, false);
    f.run(
        &["init", f.root.to_str().unwrap(), "--apply", "--format=json"],
        f.bin.to_str().unwrap(),
    );
    let report = f.lint(&[]);
    let brief = &report["next"]["repair_brief"];
    assert_eq!(brief["reason_code"], "ruff_local_tool_invalid", "{report}");
    assert!(
        brief["step"].as_str().unwrap().contains(".venv/bin/ruff"),
        "{brief}"
    );
    let id = brief["task_id"].as_str().unwrap();
    let task = fs::read_to_string(f.root.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
    assert!(task.contains(".venv/bin/ruff"), "{task}");
    let repeated = f.lint(&[]);
    assert_eq!(repeated["next"]["repair_brief"]["task_id"], id);
    assert_eq!(repeated["backlog_sync"]["new_blockers"], 0);
    let verified = f.run(
        &[
            "task",
            "verify",
            id,
            f.root.to_str().unwrap(),
            "--format=json",
        ],
        f.bin.to_str().unwrap(),
    );
    assert_eq!(verified["observation"], "still_blocked", "{verified}");
    assert!(!f.bin.join("started").exists());
}
