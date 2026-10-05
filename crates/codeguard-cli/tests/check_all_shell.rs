#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn new() -> Self {
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-shell-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("a.sh"), "#!/bin/bash\necho $1\n").unwrap();
        fs::write(p.join("b.sh"), "#!/bin/bash\necho $1\n").unwrap();
        Self(p)
    }
    fn tool(&self) -> PathBuf {
        let p = self.0.join("shellcheck");
        fs::write(&p,"#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: 0.11.0\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '%s\\n' '{\"comments\":[{\"file\":\"-\",\"line\":2,\"endLine\":2,\"column\":6,\"endColumn\":8,\"level\":\"info\",\"code\":2086,\"message\":\"private\",\"fix\":null}]}'\nexit 1\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn run(&self, selection: &str, tool: Option<&PathBuf>) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(["check", selection])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", &self.0);
        if let Some(t) = tool {
            c.arg("--shellcheck-tool").arg(t);
        }
        let o = c.output().unwrap();
        assert_eq!(o.status.code(), Some(3), "{o:?}");
        serde_json::from_slice(&o.stdout).unwrap()
    }
    fn init(&self) {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&self.0)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
    }
}
#[test]
fn shell_project_scan_preserves_per_file_rules_and_stable_tasks() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let first = p.run("shell", Some(&tool));
    assert_eq!(first["schema_version"], "0.52.0");
    let scan = &first["native_results"]["shell_lint"];
    assert_eq!(scan["files"].as_array().unwrap().len(), 2);
    assert_eq!(scan["local_check_complete"], true);
    for f in scan["files"].as_array().unwrap() {
        assert_eq!(f["dialect"], "bash");
        assert_eq!(f["native"]["diagnostics"][0]["rule_id"], "SC2086");
        assert_eq!(f["workbench"]["status"], "synced_partial");
    }
    let tasks = fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count();
    assert_eq!(tasks, 2);
    p.run("shell", Some(&tool));
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        tasks
    );
    assert_eq!(
        first["next"]["repair_brief"]["checker_id"],
        "shell.shellcheck"
    );
    assert_eq!(first["delivery_decision"], "not_evaluated");
}
#[test]
fn missing_shell_tool_keeps_files_incomplete_without_initializing() {
    let p = Project::new();
    let r = p.run("all", None);
    assert_eq!(
        r["native_results"]["shell_lint"]["local_check_complete"],
        false
    );
    assert_eq!(
        r["native_results"]["shell_lint"]["files"][0]["native"]["reason"],
        "shellcheck_tool_not_found"
    );
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn shell_project_tasks_bind_request_root_instead_of_nested_workbench() {
    let p = Project::new();
    let nested = p.0.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("c.sh"), "#!/bin/bash\necho $1\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&nested)
        .arg("--apply")
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    p.init();
    let tool = p.tool();
    let report = p.run("shell", Some(&tool));
    assert_eq!(
        report["native_results"]["shell_lint"]["files"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        3
    );
    assert_eq!(
        fs::read_dir(nested.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn shell_project_preserves_unsupported_and_unknown_dialects() {
    let p = Project::new();
    fs::write(p.0.join("a.sh"), "echo $1\n").unwrap();
    fs::write(p.0.join("b.sh"), "#!/bin/zsh\necho $1\n").unwrap();
    p.init();
    let tool = p.tool();
    let report = p.run("shell", Some(&tool));
    let files = report["native_results"]["shell_lint"]["files"]
        .as_array()
        .unwrap();
    assert_eq!(files[0]["native"]["reason"], "shell_dialect_unresolved");
    assert_eq!(files[1]["native"]["reason"], "shell_dialect_unsupported");
    assert_eq!(
        report["native_results"]["shell_lint"]["local_check_complete"],
        false
    );
    assert_eq!(report["next"]["disposition"], "needs_decision");
}
#[test]
fn shell_project_limit_is_visible_and_wrong_language_flags_fail_before_work() {
    let p = Project::new();
    for i in 0..65 {
        fs::write(p.0.join(format!("file{i}.sh")), "#!/bin/bash\necho $1\n").unwrap();
    }
    let tool = p.tool();
    let report = p.run("shell", Some(&tool));
    let scan = &report["native_results"]["shell_lint"];
    assert_eq!(scan["files"].as_array().unwrap().len(), 64);
    assert_eq!(scan["unobserved_count"], 3);
    assert_eq!(scan["local_check_complete"], false);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python"])
        .arg(&p.0)
        .arg("--shellcheck-tool")
        .arg(&tool)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!p.0.join(".codeguard").exists());
}

#[test]
fn shell_project_source_mutation_withdraws_current_evidence() {
    let p = Project::new();
    let tool = p.tool();
    let script = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        script.replace(
            "/bin/cat >/dev/null",
            &format!(
                "/bin/echo changed > '{}'\n/bin/cat >/dev/null",
                p.0.join("b.sh").display()
            ),
        ),
    )
    .unwrap();
    let report = p.run("shell", Some(&tool));
    let scan = &report["native_results"]["shell_lint"];
    assert_eq!(scan["scope_stable"], false, "{report}");
    assert_eq!(scan["local_check_complete"], false);
    assert!(
        scan["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["input_stable"] == false)
    );
}

#[test]
fn explicit_shell_default_does_not_override_a_declared_unsupported_dialect() {
    let p = Project::new();
    fs::write(p.0.join("a.sh"), "# project default\necho $1\n").unwrap();
    fs::write(p.0.join("b.sh"), "#!/bin/zsh\necho $1\n").unwrap();
    let tool = p.tool();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "shell"])
        .arg(&p.0)
        .arg("--shellcheck-tool")
        .arg(&tool)
        .args(["--shell-dialect", "bash", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    let f = &r["native_results"]["shell_lint"]["files"];
    assert_eq!(f[0]["dialect"], "bash");
    assert_eq!(f[0]["native"]["status"], "diagnostics_observed");
    assert_eq!(f[1]["dialect"], "zsh");
    assert_eq!(f[1]["native"]["reason"], "shell_dialect_unsupported");
}

#[test]
fn cancelled_project_scan_does_not_create_environment_repair_tasks() {
    let p = Project::new();
    p.init();
    let tool = p.tool();
    let marker = p.0.join("started");
    fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then /usr/bin/touch '{}'; /bin/sleep 5; fi\nexit 0\n",marker.display())).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "shell"])
        .arg(&p.0)
        .arg("--shellcheck-tool")
        .arg(&tool)
        .args(["--timeout", "2s", "--format=json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..200 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(marker.exists());
    assert!(
        Command::new("/bin/kill")
            .arg("-INT")
            .arg(child.id().to_string())
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{output:?}");
    let r: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(r["command_status"], "cancelled");
    assert_eq!(r["execution_budget"]["started_native_task_count"], 1);
    assert_eq!(
        fs::read_dir(p.0.join(".codeguard/tasks")).unwrap().count(),
        0,
        "{r}"
    );
}
