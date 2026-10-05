#![cfg(unix)]
use codeguard_cli::rust_lint_scan::observe_cargo_clippy;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-clippy-input-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname=\"sample\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn answer() -> i32 { return 42; }\n",
        )
        .unwrap();
        Self(root)
    }
    fn scan(&self, mutation: &str, finding: bool) -> Value {
        let tool = self.0.join("cargo-probe");
        let message = if finding {
            "printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}'\n"
        } else {
            ""
        };
        fs::write(&tool,format!("#!/bin/sh\n{mutation}\n{message}printf '%s\\n' '{{\"reason\":\"build-finished\",\"success\":true}}'\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let report = observe_cargo_clippy(
            &self.0,
            &BTreeSet::from(["src/lib.rs".into()]),
            Some(&tool),
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false),
        );
        if let Ok(dir) = std::env::var("CODEGUARD_CLIPPY_REPORT_DIR") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                PathBuf::from(dir).join(format!(
                    "report-{}.json",
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        report
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn changed_source_does_not_bind_old_diagnostic_to_new_bytes() {
    let p = Project::new();
    let report = p.scan(
        "printf '%s\\n' 'pub fn replacement() -> i32 { 99 }' > src/lib.rs",
        true,
    );
    assert_eq!(report["local_scan_complete"], false, "{report}");
    assert_eq!(report["reason"], "rust_inputs_changed_during_scan");
    assert_eq!(report["findings"], serde_json::json!([]));
}
#[test]
fn changed_config_or_new_optional_config_cannot_produce_clean_observation() {
    for (name, before) in [
        ("clippy.toml", true),
        (".clippy.toml", false),
        ("Cargo.lock", true),
        ("rust-toolchain.toml", false),
        (".cargo/config.toml", false),
    ] {
        let p = Project::new();
        if name.starts_with(".cargo/") {
            fs::create_dir(p.0.join(".cargo")).unwrap();
        }
        if before {
            fs::write(p.0.join(name), "# before\n").unwrap();
        }
        let report = p.scan(&format!("printf '%s\\n' '# changed' > {name}"), false);
        assert_eq!(report["local_scan_complete"], false, "{name}: {report}");
        assert_eq!(report["reason"], "rust_inputs_changed_during_scan");
    }
}
#[test]
fn changed_tool_cannot_supply_current_source_finding() {
    let p = Project::new();
    let report = p.scan("printf '%s\\n' '# changed tool' >> \"$0\"", true);
    assert_eq!(report["local_scan_complete"], false, "{report}");
    assert_eq!(report["reason"], "cargo_tool_changed_during_scan");
    assert_eq!(report["findings"], serde_json::json!([]));
}
#[test]
fn stable_native_diagnostic_keeps_original_source_binding() {
    let p = Project::new();
    let report = p.scan("", true);
    assert_eq!(report["local_scan_complete"], true);
    assert_eq!(report["findings"][0]["rule_id"], "clippy::needless_return");
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn stable_partial_native_report_keeps_valid_diagnostic() {
    let p = Project::new();
    let report = p.scan("printf '%s\\n' '{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::needless_return\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}]}}' 'bad-json'", false);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["findings"][0]["rule_id"], "clippy::needless_return");
}

#[test]
#[ignore = "requires an installed native Cargo Clippy; set CODEGUARD_CARGO_BIN"]
fn real_clippy_output_is_withdrawn_when_source_changes_after_native_execution() {
    let tool = std::env::var("CODEGUARD_CARGO_BIN").expect("原生 Cargo 路径");
    assert!(std::path::Path::new(&tool).is_absolute());
    let quoted = format!("'{}'", tool.replace('\'', "'\\''"));
    let p = Project::new();
    let wrapper = p.0.join("cargo-original");
    fs::write(&wrapper,format!("#!/bin/sh\n{quoted} \"$@\" > native-output.jsonl\nresult=$?\ncat native-output.jsonl\nif [ \"$result\" = 0 ]; then printf '%s\\n' 'pub fn replacement() -> i32 {{ 99 }}' > src/lib.rs; fi\nexit \"$result\"\n")).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let report = observe_cargo_clippy(
        &p.0,
        &BTreeSet::from(["src/lib.rs".into()]),
        Some(&wrapper),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    );
    let raw = fs::read(p.0.join("native-output.jsonl")).unwrap();
    let native = codeguard_adapters::parse_cargo_clippy_json(&raw);
    assert!(native.issue.is_none());
    assert!(
        native
            .findings
            .iter()
            .any(|f| f.rule_id == "clippy::needless_return")
    );
    assert_eq!(report["reason"], "rust_inputs_changed_during_scan");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["findings"], serde_json::json!([]));
    if let Ok(dir) = std::env::var("CODEGUARD_CLIPPY_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            PathBuf::from(dir).join("real-clippy-stale-source.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn unavailable_source_is_rejected_before_native_startup() {
    let p = Project::new();
    fs::remove_file(p.0.join("src/lib.rs")).unwrap();
    let report = p.scan("printf '%s\\n' executed > executed-marker", false);
    assert_eq!(report["reason"], "rust_inputs_unavailable");
    assert!(!p.0.join("executed-marker").exists());
}

#[test]
fn native_clippy_uses_locked_dependency_resolution() {
    let p = Project::new();
    let report = p.scan("printf '%s\\n' \"$@\" > native-argv", true);
    assert_eq!(report["local_scan_complete"], true);
    let argv = fs::read_to_string(p.0.join("native-argv")).unwrap();
    assert!(argv.lines().any(|v| v == "--locked"));
    assert!(
        report["recheck_command"]
            .as_str()
            .unwrap()
            .contains("--locked")
    );
}

#[test]
fn missing_lock_is_preparation_failure_before_native_startup() {
    let p = Project::new();
    fs::remove_file(p.0.join("Cargo.lock")).unwrap();
    let report = p.scan("printf '%s\\n' executed > executed-marker", false);
    assert_eq!(report["reason"], "cargo_lock_unavailable");
    assert!(!p.0.join("executed-marker").exists());
}

#[test]
fn cancellation_keeps_priority_when_native_tool_changed_source() {
    let p = Project::new();
    let tool = p.0.join("cargo-cancel");
    fs::write(&tool, "#!/bin/sh\nprintf '%s\\n' 'pub fn changed() -> i32 { 99 }' > src/lib.rs\ntouch cancellation-ready\nexec sleep 10\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let cancelled = AtomicBool::new(false);
    let report = std::thread::scope(|scope| {
        scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !p.0.join("cancellation-ready").exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            cancelled.store(true, Ordering::SeqCst);
        });
        observe_cargo_clippy(
            &p.0,
            &BTreeSet::from(["src/lib.rs".into()]),
            Some(&tool),
            Instant::now() + Duration::from_secs(8),
            &cancelled,
        )
    });
    assert_eq!(report["reason"], "request_cancelled", "{report}");
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["findings"], serde_json::json!([]));
}
