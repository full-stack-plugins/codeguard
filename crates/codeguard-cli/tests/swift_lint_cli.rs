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
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-swift-lint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("App.swift"), source).unwrap();
        Self(root)
    }
    fn tool(&self, body: &str, version: &str) -> PathBuf {
        let p = self.0.join("swiftc");
        fs::write(&p,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'Apple Swift version {version} (fixture)'; exit 0; fi\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        c.args(["lint", "swift"])
            .arg(self.0.join("App.swift"))
            .env("PATH", &self.0);
        c
    }
    fn report(&self, args: &[&str]) -> Value {
        let out = self
            .command()
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[test]
fn swift_native_positions_reach_json_and_human_without_claiming_project_success() {
    let p = Project::new("func f(_ x: ) {}\n");
    let tool = p.tool(
        "printf '<stdin>:1:13: error: expected type\\n' >&2; exit 1",
        "6.4",
    );
    let report = p.report(&["--swift-tool", tool.to_str().unwrap()]);
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(
        report["native"]["diagnostics"][0]["rule_id"],
        "swift.parse.error"
    );
    assert_eq!(report["syntax_precheck"], Value::Null);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let out = p
        .command()
        .args(["--swift-tool", tool.to_str().unwrap(), "--format=human"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("swift.parse.error") && text.contains(":1:13"),
        "{text}"
    );
}
#[test]
fn first_path_compiler_is_used_and_selected_failures_do_not_switch_to_wasm() {
    let p = Project::new("struct C {}\n");
    p.tool("exit 0", "6.4");
    let r = p.report(&[]);
    assert_eq!(r["tool_selection"]["source"], "path");
    assert_eq!(r["native"]["status"], "completed");
    assert_eq!(r["syntax_precheck"], Value::Null);
    p.tool("exit 0", "6.3");
    let r = p.report(&[]);
    assert_eq!(
        r["native"]["reason"],
        "swift_version_unverified_or_unsupported"
    );
    assert_eq!(r["syntax_precheck"], Value::Null);
    assert_eq!(r["setup"]["native_confirmation_required"], true);
    let r = p.report(&["--swift-tool", p.0.join("missing").to_str().unwrap()]);
    assert_eq!(r["native"]["status"], "incomplete");
    assert_eq!(r["syntax_precheck"], Value::Null);
}
#[test]
fn malformed_native_reports_and_utf8_columns_never_become_source_findings() {
    for (source, body) in [
        (
            "func f() {}\n",
            "printf 'Other.swift:1:1: error: bad\\n' >&2; exit 1",
        ),
        (
            "\"😀\"\n",
            "printf '<stdin>:1:3: error: bad\\n' >&2; exit 1",
        ),
        ("func f() {}\n", "echo 'disk full' >&2; exit 1"),
        (
            "func f() {}\n",
            "printf '<stdin>:1:1: error: bad\\n' >&2; exit 0",
        ),
    ] {
        let p = Project::new(source);
        p.tool(body, "6.4");
        let r = p.report(&[]);
        assert_eq!(r["native"]["status"], "incomplete");
        assert_eq!(r["native"]["diagnostics"], serde_json::json!([]));
        assert_eq!(r["setup"]["native_confirmation_required"], true);
    }
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn missing_swift_tool_uses_wasm_and_requires_confirmation_for_hidden_recovery() {
    let p = Project::new("func f(_ x: ) {}\n");
    let r = p.report(&[]);
    assert_eq!(r["tool_selection"]["source"], "not_found");
    assert_eq!(r["syntax_precheck"]["precheck"]["truncated_files"], 1);
    assert_eq!(r["setup"]["native_confirmation_required"], true);
    fs::write(p.0.join("App.swift"), "struct C {}\n").unwrap();
    let r = p.report(&[]);
    assert_eq!(r["syntax_precheck"]["precheck"]["truncated_files"], 0);
    assert_eq!(r["setup"]["native_confirmation_required"], false);
    assert_eq!(r["delivery_decision"], "not_evaluated");
}
#[cfg(not(feature = "wasm-precheck"))]
#[test]
fn missing_both_backends_requires_native_confirmation() {
    let p = Project::new("struct C {}\n");
    let r = p.report(&[]);
    assert_eq!(r["syntax_precheck"]["reason"], "wasm_feature_not_built");
    assert_eq!(r["setup"]["native_confirmation_required"], true);
}
#[test]
fn invalid_swift_arguments_do_not_execute_the_selected_tool() {
    let p = Project::new("struct C {}\n");
    let marker = p.0.join("executed");
    let tool = p.tool(&format!("echo started > '{}'", marker.display()), "6.4");
    for args in [
        vec!["--swift-tool", "relative"],
        vec!["--timeout", "0s"],
        vec!["--format=json", "--format=json"],
        vec![
            "--swift-tool",
            tool.to_str().unwrap(),
            "--swift-tool",
            tool.to_str().unwrap(),
        ],
    ] {
        let out = p.command().args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(2));
    }
    assert!(!marker.exists());
}

#[test]
fn swift_changed_source_or_redirected_compiler_withdraws_completion() {
    use std::os::unix::fs::symlink;
    let p = Project::new("struct C {}\n");
    p.tool(
        &format!(
            "echo changed > '{}'; exit 0",
            p.0.join("App.swift").display()
        ),
        "6.4",
    );
    let r = p.report(&[]);
    assert_eq!(r["native"]["reason"], "swift_source_changed_during_check");
    assert_eq!(r["input_stable"], false);
    assert_eq!(r["native"]["diagnostics"], serde_json::json!([]));
    let p = Project::new("struct C {}\n");
    let selected = p.0.join("swiftc");
    let first = p.0.join("first");
    let second = p.0.join("second");
    p.tool(
        &format!(
            "/bin/ln -sf '{}' '{}'; exit 0",
            second.display(),
            selected.display()
        ),
        "6.4",
    );
    fs::rename(&selected, &first).unwrap();
    fs::write(&second, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&second, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&first, &selected).unwrap();
    let r = p.report(&[]);
    assert_eq!(
        r["native"]["reason"],
        "swift_selected_tool_changed_during_check"
    );
    assert_eq!(r["native"]["diagnostics"], serde_json::json!([]));
}
#[test]
fn swift_invalid_source_and_missing_file_have_no_repair_positions() {
    let p = Project::new("struct C {}\n");
    p.tool("exit 0", "6.4");
    fs::write(p.0.join("App.swift"), [255]).unwrap();
    let r = p.report(&[]);
    assert_eq!(r["native"]["reason"], "swift_source_unavailable_or_invalid");
    assert_eq!(r["setup"]["native_confirmation_required"], true);
    fs::remove_file(p.0.join("App.swift")).unwrap();
    let r = p.report(&[]);
    assert_eq!(r["native"]["diagnostics"], serde_json::json!([]));
    assert_eq!(r["source_sha256"], Value::Null);
    assert_eq!(r["input_stable"], false);
}
#[test]
fn swift_timeout_stays_visible_without_wasm_fallback() {
    let p = Project::new("struct C {}\n");
    p.tool("/bin/sleep 30", "6.4");
    let r = p.report(&["--timeout", "100ms"]);
    assert_eq!(r["native"]["reason"], "request_deadline_exceeded");
    assert_eq!(r["syntax_precheck"], Value::Null);
    assert_eq!(r["setup"]["native_confirmation_required"], true);
}
#[test]
fn swift_sigint_returns_cancelled_feedback() {
    use std::time::{Duration, Instant};
    let p = Project::new("struct C {}\n");
    let marker = p.0.join("started");
    p.tool(
        &format!("echo started > '{}'; /bin/sleep 30", marker.display()),
        "6.4",
    );
    let child = p
        .command()
        .arg("--format=json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists());
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["command_status"], "cancelled");
    assert_eq!(r["native"]["reason"], "request_cancelled");
}
