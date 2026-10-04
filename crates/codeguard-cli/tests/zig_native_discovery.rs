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
impl Project {
    fn new() -> Self {
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-zig-discover-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("app.zig"), "const Empty = struct {};\n").unwrap();
        Self(p)
    }
    fn tool(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\n{body}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn lint(&self, extra: &[&str], path: &std::ffi::OsStr) -> Value {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "zig"])
            .arg(self.0.join("app.zig"))
            .arg("--format=json")
            .args(extra)
            .env("PATH", path)
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn installed_path_zig_is_preferred_to_wasm_and_is_reported() {
    let p = Project::new();
    p.tool(
        "zig",
        "printf called > \"$0.called\"; while IFS= read -r line; do :; done; exit 0",
    );
    let r = p.lint(&[], p.0.as_os_str());
    assert_eq!(r["native"]["status"], "completed", "{r}");
    assert_eq!(r["tool_selection"]["source"], "path");
    assert_eq!(r["source_current"], true);
    assert!(r["syntax_precheck"].is_null());
    assert!(p.0.join("zig.called").exists());
}
#[test]
fn changed_source_cannot_retain_native_diagnostics_or_parse_the_old_bytes() {
    let p = Project::new();
    let tool=p.tool("zig",&format!("while IFS= read -r line; do :; done; printf 'changed\\n' > '{}'; printf '<stdin>:1:3: error: failure\\n' >&2; exit 1",p.0.join("app.zig").display()));
    let r = p.lint(&["--zig-tool", tool.to_str().unwrap()], p.0.as_os_str());
    assert_eq!(r["native"]["status"], "incomplete", "{r}");
    assert_eq!(r["native"]["reason"], "zig_source_changed_during_check");
    assert_eq!(r["source_current"], false);
    assert!(r["native"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(r["syntax_precheck"].is_null());
}
#[test]
fn explicit_invalid_tool_does_not_execute_the_path_candidate() {
    let p = Project::new();
    p.tool("zig", "printf called > \"$0.called\"; exit 0");
    let missing = p.0.join("missing");
    let r = p.lint(&["--zig-tool", missing.to_str().unwrap()], p.0.as_os_str());
    assert_eq!(r["tool_selection"]["source"], "explicit");
    assert_ne!(r["native"]["status"], "completed");
    assert!(!p.0.join("zig.called").exists());
}

#[test]
fn changed_tool_alias_with_unchanged_bytes_withdraws_old_positions() {
    let p = Project::new();
    let second = p.tool("second-zig", "while IFS= read -r line; do :; done; exit 0");
    let alias = p.0.join("zig");
    let first=p.tool("first-zig",&format!("while IFS= read -r line; do :; done; /bin/ln -sf '{}' '{}'; printf '<stdin>:1:3: error: failure\\n' >&2; exit 1",second.display(),alias.display()));
    std::os::unix::fs::symlink(first, &alias).unwrap();
    let r = p.lint(&[], p.0.as_os_str());
    assert_eq!(r["native"]["status"], "incomplete");
    assert_eq!(
        r["native"]["reason"],
        "zig_tool_target_changed_during_check"
    );
    assert!(r["native"]["diagnostics"].as_array().unwrap().is_empty());
    assert!(r["syntax_precheck"].is_null());
    assert_eq!(r["source_current"], true);
}

#[test]
fn first_selected_version_failure_does_not_execute_a_later_zig() {
    let p = Project::new();
    let first_dir = p.0.join("old");
    fs::create_dir(&first_dir).unwrap();
    let old = first_dir.join("zig");
    fs::write(&old, "#!/bin/sh\nprintf '0.15.0\\n'; exit 0\n").unwrap();
    fs::set_permissions(&old, fs::Permissions::from_mode(0o700)).unwrap();
    p.tool("zig", "printf called > \"$0.called\"; exit 0");
    let path = std::env::join_paths([first_dir, p.0.clone()]).unwrap();
    let r = p.lint(&[], &path);
    assert_eq!(
        r["native"]["reason"],
        "zig_version_unverified_or_unsupported"
    );
    assert!(!p.0.join("zig.called").exists());
    assert_eq!(r["delivery_decision"], "not_evaluated");
}

#[test]
fn relative_empty_and_nonexecutable_path_entries_are_not_selected() {
    let p = Project::new();
    let bad = p.0.join("nonexec");
    fs::create_dir(&bad).unwrap();
    fs::write(bad.join("zig"), "not executable").unwrap();
    fs::set_permissions(bad.join("zig"), fs::Permissions::from_mode(0o600)).unwrap();
    let good = p.tool("zig", "while IFS= read -r line; do :; done; exit 0");
    let path = std::env::join_paths([PathBuf::new(), PathBuf::from("relative"), bad, p.0.clone()])
        .unwrap();
    let r = p.lint(&[], &path);
    assert_eq!(r["native"]["status"], "completed");
    assert_eq!(r["tool_selection"]["executable"], serde_json::json!(good));
}

#[test]
#[ignore = "requires explicitly provided existing Zig 0.16.0; no installation"]
fn real_zig_on_path_runs_native_ast_check() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_ZIG_BIN").unwrap());
    let bin = p.0.join("bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(tool, bin.join("zig")).unwrap();
    let r = p.lint(&[], bin.as_os_str());
    assert_eq!(r["native"]["status"], "completed", "{r}");
    assert_eq!(r["native"]["version"], "0.16.0");
    assert_eq!(r["tool_selection"]["source"], "path");
    assert!(r["syntax_precheck"].is_null());
    if let Ok(path) = std::env::var("CODEGUARD_ZIG_DISCOVERY_REPORT") {
        fs::write(p.0.join("app.zig"), "pub fn main( void {\n").unwrap();
        let bad = p.lint(&[], bin.as_os_str());
        assert_eq!(bad["native"]["status"], "diagnostics_observed", "{bad}");
        assert!(
            bad["native"]["diagnostic_count"]
                .as_u64()
                .is_some_and(|n| n > 0)
        );
        assert!(bad["syntax_precheck"].is_null());
        fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({"clean":r,"broken":bad})).unwrap(),
        )
        .unwrap();
    }
}
