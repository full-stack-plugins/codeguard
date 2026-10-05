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
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-ruby-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.rb"), "def f(\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("ruby");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruby {version} (fixture) [test]\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '%s\\n' '-:1: syntax error, unexpected end-of-input' >&2\nexit 1\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, language: &str, tool: Option<&PathBuf>) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command
            .args(["check", language])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"])
            .env("PATH", &self.0);
        if let Some(tool) = tool {
            command.arg("--ruby-tool").arg(tool);
        }
        let out = command.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn init(&self) {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init"])
            .arg(&self.0)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
    }
}
#[test]
fn ruby_native_project_scan_preempts_wasm_without_fabricating_columns() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    let report = p.check("all", Some(&tool));
    let scan = &report["native_results"]["ruby_lint"];
    assert_eq!(report["schema_version"], "0.51.0");
    assert_eq!(scan["files"][0]["native"]["status"], "diagnostics_observed");
    assert_eq!(scan["files"][0]["native"]["diagnostics"][0]["line"], 1);
    assert!(
        scan["files"][0]["native"]["diagnostics"][0]
            .get("column")
            .is_none()
    );
    assert_eq!(report["execution_budget"]["native_task_count"], 1);
    assert_eq!(report["execution_budget"]["started_native_task_count"], 1);
    assert_eq!(report["delivery_decision"], "incomplete");
    #[cfg(feature = "wasm-precheck")]
    assert!(
        !report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["language"] == "ruby")
    );
}
#[test]
fn ruby_path_version_failure_never_falls_back_to_wasm() {
    let p = Project::new();
    p.tool("3.4.0");
    let report = p.check("ruby", None);
    assert_eq!(
        report["native_results"]["ruby_lint"]["tool_selection"]["source"],
        "path"
    );
    assert_eq!(
        report["native_results"]["ruby_lint"]["files"][0]["native"]["reason"],
        "ruby_syntax_version_unverified"
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        !report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["language"] == "ruby")
    );
}
#[test]
fn missing_ruby_keeps_bounded_candidate_fallback() {
    let p = Project::new();
    let report = p.check("all", None);
    assert_eq!(
        report["native_results"]["ruby_lint"]["tool_selection"]["source"],
        "not_found"
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["language"] == "ruby")
    );
}
#[test]
fn ruby_project_and_standalone_lint_reuse_native_task_identity() {
    let p = Project::new();
    p.init();
    let tool = p.tool("2.6.10p210");
    let report = p.check("ruby", Some(&tool));
    let id = report["native_results"]["ruby_lint"]["files"][0]["task_id"]
        .as_str()
        .unwrap();
    assert_eq!(report["next"]["schema_version"], "0.15.0");
    assert_eq!(report["next"]["repair_brief"]["task_id"], id);
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "ruby"])
        .arg(p.0.join("app.rb"))
        .arg("--ruby-tool")
        .arg(tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let lint: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(lint["task_id"], id);
}

#[test]
fn ruby_changed_source_withdraws_old_diagnostics() {
    let p = Project::new();
    let tool = p.tool("2.6.10p210");
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace(
            "/bin/cat >/dev/null",
            &format!(
                "/bin/cat >/dev/null\nprintf 'puts 1\\n' > '{}'",
                p.0.join("app.rb").display()
            ),
        ),
    )
    .unwrap();
    let report = p.check("all", Some(&tool));
    let file = &report["native_results"]["ruby_lint"]["files"][0];
    assert_eq!(file["current"], false);
    assert!(file["native"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(file["recheck_argv"].is_null());
    assert_eq!(
        report["native_results"]["ruby_lint"]["local_parse_complete"],
        false
    );
}
#[test]
fn ruby_scan_limit_retains_unobserved_scope_and_selected_tool_preference() {
    let p = Project::new();
    let tool = p.tool("3.4.0");
    for n in 0..64 {
        fs::write(p.0.join(format!("extra{n:02}.rb")), "puts 1\n").unwrap();
    }
    let report = p.check("all", Some(&tool));
    let scan = &report["native_results"]["ruby_lint"];
    assert_eq!(scan["source_file_count"], 65);
    assert_eq!(scan["files"].as_array().unwrap().len(), 64);
    assert_eq!(scan["unobserved_count"], 1);
    assert_eq!(scan["local_parse_complete"], false);
    #[cfg(feature = "wasm-precheck")]
    assert!(
        !report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["language"] == "ruby")
    );
}
