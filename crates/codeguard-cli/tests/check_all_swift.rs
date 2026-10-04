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
            "cg-swift-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("App.swift"), "func f(_ x: ) {}\n").unwrap();
        Self(root)
    }
    fn tool(&self, version: &str) -> PathBuf {
        let tool = self.0.join("swiftc");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'Apple Swift version {version} (swiftlang-6.4)\\nTarget: arm64-apple-macosx26.0\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '<stdin>:1:13: error: expected type\\nfunc f(_ x: ) {{}}\\n            ^\\n' >&2\nexit 1\n")).unwrap();
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
            c.arg("--swift-tool").arg(p);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[test]
fn project_check_prefers_explicit_swift_parser_and_reports_current_locations() {
    let p = Project::new();
    let tool = p.tool("6.4");
    let r = p.check(Some(&tool));
    let scan = &r["native_results"]["swift_lint"];
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
        !r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["language"] == "swift")
    );
}
#[test]
fn project_swift_path_selection_does_not_fall_back_after_unsupported_version() {
    let p = Project::new();
    p.tool("5.9");
    let r = p.check(None);
    assert_eq!(
        r["native_results"]["swift_lint"]["tool_selection"]["source"],
        "path"
    );
    assert_eq!(
        r["native_results"]["swift_lint"]["files"][0]["native"]["status"],
        "incomplete"
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        !r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["language"] == "swift")
    );
}

#[test]
fn changed_swift_source_withdraws_native_positions() {
    let p = Project::new();
    let tool = p.tool("6.4");
    let body = fs::read_to_string(&tool).unwrap();
    fs::write(
        &tool,
        body.replace(
            "printf '<stdin>",
            &format!(
                "printf 'struct Changed {{}}\\n' > '{}'\nprintf '<stdin>",
                p.0.join("App.swift").display()
            ),
        ),
    )
    .unwrap();
    let r = p.check(Some(&tool));
    let f = &r["native_results"]["swift_lint"]["files"][0];
    assert_eq!(f["current"], false, "{r}");
    assert_eq!(f["native"]["reason"], "swift_source_changed_after_check");
    assert!(f["native"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(f["recheck_argv"].is_null());
}

#[test]
fn swift_scan_limit_remains_unobserved_without_selected_tool_fallback() {
    let p = Project::new();
    let tool = p.tool("5.9");
    for i in 0..64 {
        fs::write(p.0.join(format!("Extra{i:02}.swift")), "struct X {}\n").unwrap();
    }
    let r = p.check(Some(&tool));
    let scan = &r["native_results"]["swift_lint"];
    assert_eq!(scan["source_file_count"], 65);
    assert_eq!(scan["files"].as_array().unwrap().len(), 64);
    assert_eq!(scan["unobserved_count"], 1);
    assert_eq!(scan["local_parse_complete"], false);
    #[cfg(feature = "wasm-precheck")]
    assert!(
        !r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["language"] == "swift")
    );
}

#[test]
fn missing_swift_compiler_retains_candidate_fallback_and_explicit_task_gap() {
    let p = Project::new();
    let r = p.check(None);
    let scan = &r["native_results"]["swift_lint"];
    assert_eq!(scan["tool_selection"]["source"], "not_found");
    assert_eq!(scan["task_status"], "not_connected");
    assert!(
        r["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "swift_native_task_connection_not_implemented")
    );
    #[cfg(feature = "wasm-precheck")]
    assert!(
        r["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["language"] == "swift")
    );
}
