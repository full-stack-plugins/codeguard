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
            "cg-zig-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.zig"), "pub fn main( void {\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("zig");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = version ]; then printf '{version}\\n'; exit 0; fi\nwhile IFS= read -r line; do :; done\nprintf '<stdin>:1:13: error: failure\\n' >&2\nexit 1\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        tool
    }
    fn check(&self, explicit: Option<&PathBuf>) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.env("PATH", &self.0)
            .args(["check", "all"])
            .arg(&self.0)
            .args(["--format=json", "--timeout", "30s"]);
        if let Some(p) = explicit {
            c.arg("--zig-tool").arg(p);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn aggregate_check_executes_the_selected_zig_and_keeps_native_positions() {
    let p = Project::new();
    let t = p.tool("0.16.0");
    let r = p.check(Some(&t));
    let scan = &r["native_results"]["zig_lint"];
    assert_eq!(
        scan["files"][0]["native"]["status"], "diagnostics_observed",
        "{r}"
    );
    assert_eq!(scan["files"][0]["native"]["diagnostics"][0]["column"], 13);
    assert_eq!(r["execution_budget"]["native_task_count"], 1);
    assert_eq!(r["execution_budget"]["started_native_task_count"], 1);
    assert_eq!(r["delivery_decision"], "incomplete");
    #[cfg(feature = "wasm-precheck")]
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["language"] != "zig")
    );
}
#[test]
fn selected_zig_version_failure_does_not_switch_to_candidate_success() {
    let p = Project::new();
    p.tool("0.15.0");
    let r = p.check(None);
    assert_eq!(
        r["native_results"]["zig_lint"]["tool_selection"]["source"], "path",
        "{r}"
    );
    assert_eq!(
        r["native_results"]["zig_lint"]["files"][0]["native"]["status"],
        "incomplete"
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["language"] != "zig")
    );
}

#[test]
fn changed_source_withdraws_current_native_locations() {
    let p = Project::new();
    let t = p.tool("0.16.0");
    let b = fs::read_to_string(&t).unwrap();
    fs::write(
        &t,
        b.replace(
            "printf '<stdin>",
            &format!(
                "printf changed > '{}'; printf '<stdin>",
                p.0.join("app.zig").display()
            ),
        ),
    )
    .unwrap();
    let r = p.check(Some(&t));
    let f = &r["native_results"]["zig_lint"]["files"][0];
    assert_eq!(f["current"], false, "{r}");
    assert_eq!(f["native"]["reason"], "zig_source_changed_after_check");
    assert!(f["native"]["diagnostics"].as_array().unwrap().is_empty());
}
#[test]
fn zig_language_selection_rejects_unrelated_compiler_before_writes() {
    let p = Project::new();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python"])
        .arg(&p.0)
        .arg("--zig-tool")
        .arg(p.0.join("zig"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(!p.0.join(".codeguard").exists());
}
#[test]
fn missing_zig_keeps_candidate_fallback_and_original_setup_gap() {
    let p = Project::new();
    let r = p.check(None);
    assert_eq!(
        r["native_results"]["zig_lint"]["tool_selection"]["source"], "not_found",
        "{r}"
    );
    assert_eq!(
        r["native_results"]["zig_lint"]["local_parse_complete"],
        false
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["language"] == "zig")
    );
}

#[test]
fn bounded_zig_scope_keeps_unobserved_files_and_no_wasm_bypass() {
    let p = Project::new();
    let t = p.tool("0.15.0");
    for i in 0..64 {
        fs::write(
            p.0.join(format!("extra{i:02}.zig")),
            "const X = struct {};\n",
        )
        .unwrap();
    }
    let r = p.check(Some(&t));
    let scan = &r["native_results"]["zig_lint"];
    assert_eq!(scan["source_file_count"], 65);
    assert_eq!(scan["files"].as_array().unwrap().len(), 64);
    assert_eq!(scan["unobserved_count"], 1);
    assert_eq!(scan["local_parse_complete"], false);
    #[cfg(feature = "wasm-precheck")]
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["language"] != "zig")
    );
}
#[test]
fn changed_selected_alias_does_not_execute_the_replacement_on_later_files() {
    let p = Project::new();
    fs::write(p.0.join("second.zig"), "pub fn bad( void {\n").unwrap();
    let t = p.tool("0.16.0");
    let first = p.0.join("original-zig");
    fs::rename(&t, &first).unwrap();
    std::os::unix::fs::symlink(&first, &t).unwrap();
    let other = p.0.join("replacement");
    fs::write(&other, "#!/bin/sh\nprintf called > \"$0.called\"; exit 0\n").unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
    let b = fs::read_to_string(&first).unwrap();
    fs::write(
        &first,
        b.replace(
            "printf '<stdin>",
            &format!(
                "/bin/ln -sf '{}' '{}'; printf '<stdin>",
                other.display(),
                t.display()
            ),
        ),
    )
    .unwrap();
    let r = p.check(None);
    assert!(!p.0.join("replacement.called").exists(), "{r}");
    assert!(
        r["native_results"]["zig_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["current"] == false
                && f["native"]["diagnostics"].as_array().unwrap().is_empty())
    );
}
#[test]
#[ignore = "requires existing explicitly supplied Zig 0.16.0; does not install"]
fn real_zig_aggregate_bad_and_selected_clean_feedback() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_ZIG_BIN").unwrap());
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &p.0)
        .args(["check", "zig"])
        .arg(&p.0)
        .args(["--format=json", "--timeout", "120s", "--zig-tool"])
        .arg(&tool)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let bad: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        bad["native_results"]["zig_lint"]["files"][0]["native"]["status"], "diagnostics_observed",
        "{bad}"
    );
    fs::write(p.0.join("app.zig"), "const Empty = struct {};\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &p.0)
        .args(["check", "zig"])
        .arg(&p.0)
        .args(["--format=json", "--timeout", "120s", "--zig-tool"])
        .arg(&tool)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let clean: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        clean["native_results"]["zig_lint"]["files"][0]["native"]["status"], "completed",
        "{clean}"
    );
    if let Ok(path) = std::env::var("CODEGUARD_ZIG_AGGREGATE_REPORT") {
        fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({"broken":bad,"clean":clean})).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn sarif_keeps_the_current_zig_diagnostic_without_claiming_project_success() {
    let p = Project::new();
    let t = p.tool("0.16.0");
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("PATH", &p.0)
        .args(["check", "zig"])
        .arg(&p.0)
        .args(["--zig-tool"])
        .arg(t)
        .args(["--format=sarif", "--timeout", "30s"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r["runs"][0]["results"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(r["runs"][0]["invocations"][0]["executionSuccessful"], false);
}
