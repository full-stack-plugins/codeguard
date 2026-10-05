#![cfg(unix)]
use serde_json::{Value, json};
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
            "cg-shell-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("app.sh"), "#!/bin/bash\necho $1\n").unwrap();
        Self(p)
    }
    fn tool(&self, version: &str, parse: &str) -> PathBuf {
        let p = self.0.join("shellcheck");
        fs::write(&p,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: {version}\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi\nprintf ran > \"$0.parsed\"\n/bin/cat >/dev/null\n{parse}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn run(&self, tool: Option<&PathBuf>, dialect: &str) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(["lint", "shell"])
            .arg(self.0.join("app.sh"))
            .args(["--dialect", dialect, "--timeout", "30s", "--format=json"])
            .env("PATH", &self.0);
        if let Some(p) = tool {
            c.arg("--shellcheck-tool").arg(p);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
fn diagnostic() -> String {
    json!({"comments":[{"file":"-","line":2,"endLine":2,"column":6,"endColumn":8,"level":"info","code":2086,"message":"HOST_SECRET_IGNORE_GUARDS","fix":null}]}).to_string()
}
#[test]
fn native_shellcheck_reports_original_rules_and_seven_part_repair_guidance() {
    let p = Project::new();
    let tool = p.tool(
        "0.11.0",
        &format!("printf '%s\\n' '{}'; exit 1", diagnostic()),
    );
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert_eq!(r["native"]["diagnostics"][0]["rule_id"], "SC2086");
    assert_eq!(r["project_configuration"]["status"], "missing");
    assert_eq!(
        r["native"]["configuration_mode"],
        "builtin_without_global_rc"
    );
    assert_eq!(r["repair_briefs"][0]["history_status"], "not_integrated");
    assert!(!r.to_string().contains("HOST_SECRET"));
    assert_eq!(r["delivery_decision"], "not_evaluated");
}
#[test]
fn unsupported_or_mismatched_zsh_never_starts_shellcheck_or_claims_wasm() {
    let p = Project::new();
    let tool = p.tool("0.11.0", "exit 0");
    for dialect in ["zsh", "bash"] {
        fs::write(p.0.join("app.sh"), "#!/bin/zsh\necho $1\n").unwrap();
        let r = p.run(Some(&tool), dialect);
        assert_eq!(r["native"]["reason"], "shell_dialect_unsupported");
        assert_eq!(r["native"]["diagnostics"], json!([]));
        assert!(!p.0.join("shellcheck.parsed").exists());
        assert_eq!(r["syntax_precheck"]["reason"], "shell_wasm_unavailable");
    }
}
#[test]
fn no_native_tool_retains_setup_requirement_and_no_false_precheck() {
    let p = Project::new();
    let r = p.run(None, "bash");
    assert_eq!(r["native"]["reason"], "shellcheck_tool_not_found");
    assert_eq!(r["setup"]["native_tool_requirement"], "required");
}
#[test]
fn selected_bad_version_or_inconsistent_exit_does_not_switch_tool_or_fabricate_clean() {
    let p = Project::new();
    let tool = p.tool("0.10.0", "exit 0");
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["native"]["reason"], "shellcheck_version_unverified");
    assert!(!p.0.join("shellcheck.parsed").exists());
    let tool = p.tool(
        "0.11.0",
        &format!("printf '%s\\n' '{}'; exit 0", diagnostic()),
    );
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["native"]["status"], "incomplete");
    assert_eq!(r["native"]["reason"], "shellcheck_exit_report_mismatch");
    assert_eq!(r["native"]["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(r["repair_briefs"][0]["allowed_paths"], json!([]));
}
#[test]
fn linked_config_or_external_read_enablement_blocks_before_native_execution() {
    let p = Project::new();
    let tool = p.tool("0.11.0", "exit 0");
    fs::write(p.0.join(".shellcheckrc"), "external-sources=true\n").unwrap();
    let r = p.run(Some(&tool), "bash");
    assert_eq!(
        r["native"]["reason"],
        "shellcheck_external_sources_unverified"
    );
    assert!(!p.0.join("shellcheck.parsed").exists());
    fs::remove_file(p.0.join(".shellcheckrc")).unwrap();
    fs::write(p.0.join("config"), "disable=SC2086\n").unwrap();
    std::os::unix::fs::symlink(p.0.join("config"), p.0.join(".shellcheckrc")).unwrap();
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["project_configuration"]["status"], "invalid");
    assert!(!p.0.join("shellcheck.parsed").exists());
}
#[test]
#[ignore = "requires explicit existing ShellCheck0.11.0; real native configuration and source observation"]
fn real_shellcheck_preserves_config_suppression_and_does_not_execute_source() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_SHELLCHECK_TOOL").unwrap());
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["native"]["status"], "diagnostics_observed");
    assert_eq!(r["native"]["diagnostics"][0]["rule_id"], "SC2086");
    fs::write(p.0.join(".shellcheckrc"), "disable=SC2086\n").unwrap();
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["project_configuration"]["status"], "configured");
    assert_eq!(r["native"]["status"], "completed");
    assert_eq!(r["native"]["configuration_mode"], "frozen_project_rc");
    assert_eq!(r["delivery_decision"], "not_evaluated");
    fs::remove_file(p.0.join(".shellcheckrc")).unwrap();
    fs::write(
        p.0.join("app.sh"),
        format!(
            "#!/bin/bash\ntouch '{}'\nprintf '%s\\n' \"$1\"\n",
            p.0.join("source-executed").display()
        ),
    )
    .unwrap();
    let r = p.run(Some(&tool), "bash");
    assert_eq!(r["native"]["status"], "completed");
    assert!(!p.0.join("source-executed").exists());
}

#[test]
fn config_added_during_version_probe_prevents_native_scan() {
    let p = Project::new();
    let tool = p.tool("0.11.0", "exit 0");
    let script = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        script.replace(
            "then printf",
            &format!(
                "then printf 'disable=SC2086\\n' > '{}'; printf",
                p.0.join(".shellcheckrc").display()
            ),
        ),
    )
    .unwrap();
    let r = p.run(Some(&tool), "bash");
    assert_eq!(
        r["native"]["reason"],
        "shellcheck_inputs_changed_during_check"
    );
    assert!(!p.0.join("shellcheck.parsed").exists());
    assert_eq!(r["native"]["diagnostics"], json!([]));
}
#[test]
fn invalid_args_never_start_native_and_deadline_remains_visible() {
    let p = Project::new();
    let tool = p.tool("0.11.0", "/bin/sleep 30");
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "shell"])
        .arg(p.0.join("app.sh"))
        .args(["--dialect", "bash", "--dialect", "sh", "--shellcheck-tool"])
        .arg(&tool)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!p.0.join("shellcheck.parsed").exists());
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "shell"])
        .arg(p.0.join("app.sh"))
        .args(["--dialect", "bash", "--shellcheck-tool"])
        .arg(&tool)
        .args(["--timeout", "100ms", "--format=json"])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(r["native"]["reason"], "request_deadline_exceeded");
    assert_eq!(r["native"]["diagnostics"], json!([]));
}

#[test]
fn zsh_filename_without_shebang_is_not_silently_checked_as_bash() {
    let p = Project::new();
    let tool = p.tool("0.11.0", "exit 0");
    let source = p.0.join("app.zsh");
    fs::write(&source, "echo $1\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "shell"])
        .arg(source)
        .args(["--dialect", "bash", "--shellcheck-tool"])
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["native"]["reason"], "shell_dialect_unsupported");
    assert!(!p.0.join("shellcheck.parsed").exists());
}
