#![cfg(unix)]

use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cg-cargo-audit-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='audit-example'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.lock"), "version = 3\n\n[[package]]\nname = 'time'\nversion = '0.1.40'\nsource = 'registry+https://github.com/rust-lang/crates.io-index'\nchecksum = '0000000000000000000000000000000000000000000000000000000000000000'\n").unwrap();
        fs::create_dir(root.join("advisory-db")).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn run(&self, tool: &std::path::Path) -> (i32, Value) {
        self.run_with_db(tool, &self.0.join("advisory-db"))
    }
    fn run_with_timeout(&self, tool: &std::path::Path, timeout: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "rust",
                self.0.to_str().unwrap(),
                "--cargo-audit-tool",
                tool.to_str().unwrap(),
                "--db",
                self.0.join("advisory-db").to_str().unwrap(),
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
    fn run_with_db(&self, tool: &std::path::Path, db: &std::path::Path) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "rust",
                self.0.to_str().unwrap(),
                "--cargo-audit-tool",
                tool.to_str().unwrap(),
                "--db",
                db.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

#[test]
#[ignore = "需要显式指定本机 cargo-audit 和离线 RustSec 数据库"]
fn real_cargo_audit_finds_vulnerable_locked_package_without_granting_delivery() {
    let tool = std::env::var("CODEGUARD_CARGO_AUDIT_BIN").expect("指定原生 cargo-audit");
    let db = std::env::var("CODEGUARD_CARGO_AUDIT_DB").expect("指定离线 RustSec 数据库");
    let fixture = Fixture::new();
    let (exit, report) =
        fixture.run_with_db(std::path::Path::new(&tool), std::path::Path::new(&db));
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], true, "{report}");
    assert_eq!(report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["reason"], "database_freshness_unverified");
    fs::write(
        fixture.0.join("Cargo.lock"),
        "version = 3\n\n[[package]]\nname='audit-example'\nversion='0.1.0'\n",
    )
    .unwrap();
    let (_, clean) = fixture.run_with_db(std::path::Path::new(&tool), std::path::Path::new(&db));
    assert_eq!(clean["local_scan_complete"], true, "{clean}");
    assert_eq!(clean["findings"], serde_json::json!([]));
    assert_eq!(clean["database_freshness"], "unverified");
    if let Ok(path) = std::env::var("CODEGUARD_TEST_CARGO_AUDIT_DATABASE_NATIVE_EVIDENCE") {
        fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({"vulnerable":report,"clean":clean}))
                .unwrap(),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_vulnerability_is_visible_but_unknown_database_never_passes() {
    let fixture = Fixture::new();
    let tool = fixture.0.join("cargo-audit-test");
    let script = "#!/bin/sh\nprintf '%s\\n' '{\"database\":{\"advisory-count\":1,\"last-commit\":null,\"last-updated\":null},\"lockfile\":{\"dependency-count\":1},\"settings\":{\"target_arch\":[],\"target_os\":[],\"severity\":null,\"ignore\":[],\"informational_warnings\":[]},\"vulnerabilities\":{\"found\":true,\"count\":1,\"list\":[{\"advisory\":{\"id\":\"RUSTSEC-2020-0071\",\"package\":\"time\",\"aliases\":[\"CVE-2020-26235\"],\"cvss\":null},\"package\":{\"name\":\"time\",\"version\":\"0.1.40\",\"source\":\"registry+https://github.com/rust-lang/crates.io-index\",\"checksum\":null}}]},\"warnings\":{}}'\nexit 1\n";
    fs::write(
        &tool,
        script.replace(
            "\"checksum\":null",
            "\"checksum\":\"0000000000000000000000000000000000000000000000000000000000000000\"",
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, report) = fixture.run(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], true);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071");
    assert_eq!(report["findings"][0]["package_version"], "0.1.40");
    assert_eq!(report["reason"], "database_freshness_unverified");
}

#[test]
fn valid_advisory_after_native_crash_remains_visible_with_incomplete_status() {
    let fixture = Fixture::new();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let tool = fixture.0.join("cargo-audit-crash-test");
    let payload = serde_json::json!({
        "database":{"advisory-count":1,"last-commit":null,"last-updated":null},
        "lockfile":{"dependency-count":1},
        "settings":{"target_arch":[],"target_os":[],"severity":null,"ignore":[],"informational_warnings":[]},
        "vulnerabilities":{"found":true,"count":1,"list":[{
            "advisory":{"id":"RUSTSEC-2020-0071","package":"time","aliases":["CVE-2020-26235"],"cvss":null},
            "package":{"name":"time","version":"0.1.40","source":"registry+https://github.com/rust-lang/crates.io-index","checksum":"0000000000000000000000000000000000000000000000000000000000000000"}
        }]},
        "warnings":{}
    });
    fs::write(
        &tool,
        format!("#!/bin/sh\nprintf '%s' '{payload}'\nexit 2\n"),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, report) = fixture.run(&tool);
    assert_eq!(exit, 3);
    assert_eq!(report["local_scan_complete"], false);
    assert_eq!(report["reason"], "cargo_audit_execution_incomplete");
    assert_eq!(report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(
        report["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|text| text.contains("已同步稳定待处理任务")))
    );
    let tasks: Vec<_> = fs::read_dir(fixture.0.join(".codeguard/tasks"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            fs::read_to_string(entry.path()).is_ok_and(|body| body.contains("Rust CVE 检查待处理"))
        })
        .collect();
    assert_eq!(tasks.len(), 1);
    let broken = fixture.0.join("cargo-audit-broken-test");
    fs::write(
        &broken,
        "#!/bin/sh\nprintf '%s' '{\"vulnerabilities\":'\nexit 2\n",
    )
    .unwrap();
    fs::set_permissions(&broken, fs::Permissions::from_mode(0o700)).unwrap();
    let (broken_exit, broken_report) = fixture.run(&broken);
    assert_eq!(broken_exit, 3);
    assert_eq!(broken_report["local_scan_complete"], false);
    assert_eq!(broken_report["findings"], serde_json::json!([]));

    let timed_out = fixture.0.join("cargo-audit-timeout-test");
    fs::write(
        &timed_out,
        format!("#!/bin/sh\nprintf '%s' '{payload}'\n/bin/sleep 5\n"),
    )
    .unwrap();
    fs::set_permissions(&timed_out, fs::Permissions::from_mode(0o700)).unwrap();
    let (timeout_exit, timeout_report) = fixture.run_with_timeout(&timed_out, "2s");
    assert_eq!(timeout_exit, 3);
    assert_eq!(timeout_report["local_scan_complete"], false);
    assert_eq!(timeout_report["reason"], "request_deadline_exceeded");
    assert_eq!(
        timeout_report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071",
        "{timeout_report}"
    );
    assert!(
        timeout_report["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|text| text.contains("已同步稳定待处理任务")))
    );

    let output_limited = fixture.0.join("cargo-audit-output-limit-test");
    fs::write(
        &output_limited,
        format!("#!/bin/sh\nprintf '%s' '{payload}'\n/bin/sleep 1\n/bin/dd if=/dev/zero bs=1048576 count=17 1>&2 2>/dev/null\n/bin/sleep 5\n"),
    )
    .unwrap();
    fs::set_permissions(&output_limited, fs::Permissions::from_mode(0o700)).unwrap();
    let (limited_exit, limited_report) = fixture.run(&output_limited);
    assert_eq!(limited_exit, 3);
    assert_eq!(limited_report["local_scan_complete"], false);
    assert_eq!(limited_report["reason"], "cargo_audit_output_limit");
    assert_eq!(
        limited_report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071",
        "{limited_report}"
    );
    assert!(
        limited_report["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|text| text.contains("已同步稳定待处理任务")))
    );

    let truncated = fixture.0.join("cargo-audit-truncated-test");
    fs::write(
        &truncated,
        "#!/bin/sh\nprintf '%s' '{\"vulnerabilities\":'\n/bin/sleep 1\n/bin/dd if=/dev/zero bs=1048576 count=17 1>&2 2>/dev/null\n/bin/sleep 5\n",
    )
    .unwrap();
    fs::set_permissions(&truncated, fs::Permissions::from_mode(0o700)).unwrap();
    let (truncated_exit, truncated_report) = fixture.run(&truncated);
    assert_eq!(truncated_exit, 3);
    assert_eq!(truncated_report["local_scan_complete"], false);
    assert_eq!(truncated_report["reason"], "cargo_audit_output_limit");
    assert_eq!(truncated_report["findings"], serde_json::json!([]));
}

#[test]
fn changed_database_keeps_candidate_but_never_grants_local_completion() {
    let mut evidence = Vec::new();
    for (name, mutation, reason, complete) in [
        (
            "rewrite",
            "printf 'changed advisory\\n' > advisory-db/crates/time/advisory.md",
            "cargo_audit_database_changed",
            false,
        ),
        (
            "remove",
            "rm advisory-db/crates/time/advisory.md",
            "cargo_audit_database_changed",
            false,
        ),
        (
            "add",
            "printf 'new advisory\\n' > advisory-db/crates/time/new.md",
            "cargo_audit_database_changed",
            false,
        ),
        (
            "replace_root",
            "mv advisory-db saved-db; mkdir -p advisory-db/crates/time; cp saved-db/crates/time/advisory.md advisory-db/crates/time/advisory.md",
            "cargo_audit_database_changed",
            false,
        ),
        (
            "revert_content",
            "printf 'temporary\\n' > advisory-db/crates/time/advisory.md; printf 'original advisory\\n' > advisory-db/crates/time/advisory.md",
            "cargo_audit_database_changed",
            false,
        ),
        (
            "lock_housekeeping",
            "touch advisory-db/db.lock",
            "database_freshness_unverified",
            true,
        ),
    ] {
        let fixture = Fixture::new();
        let tool = fixture.0.join("audit-tool");
        let database = fixture.0.join("advisory-db/crates/time");
        fs::create_dir_all(&database).unwrap();
        fs::write(database.join("advisory.md"), "original advisory\n").unwrap();
        // 运输只模拟有效原生JSON，实际改变目录输入；不证明RustSec语义。
        let output = serde_json::json!({"database":{"advisory-count":1,"last-commit":null,"last-updated":null},"lockfile":{"dependency-count":1},"settings":{"target_arch":[],"target_os":[],"severity":null,"ignore":[],"informational_warnings":[]},"vulnerabilities":{"found":true,"count":1,"list":[{"advisory":{"id":"RUSTSEC-2020-0071","package":"time","aliases":["CVE-2020-26235"],"cvss":null},"package":{"name":"time","version":"0.1.40","source":"registry+https://github.com/rust-lang/crates.io-index","checksum":"0000000000000000000000000000000000000000000000000000000000000000"}}]},"warnings":{}});
        fs::write(
            &tool,
            format!(
                "#!/bin/sh\n{mutation}\nprintf '%s\\n' '{}'\nexit 1\n",
                output
            ),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", "--apply", "--format=json"])
            .arg(&fixture.0)
            .output()
            .unwrap();
        assert_eq!(init.status.code(), Some(3));
        let (exit, report) = fixture.run(&tool);
        assert_eq!(exit, 3);
        assert_eq!(report["local_scan_complete"], complete, "{name}: {report}");
        assert_eq!(report["reason"], reason, "{name}: {report}");
        assert_eq!(report["findings"][0]["advisory_id"], "RUSTSEC-2020-0071");
        assert_eq!(report["database_freshness"], "unverified");
        if name == "revert_content" {
            assert_eq!(
                fs::read(database.join("advisory.md")).unwrap(),
                b"original advisory\n"
            );
        }
        let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["next", "--format=json"])
            .arg(&fixture.0)
            .output()
            .unwrap();
        let next: Value = serde_json::from_slice(&next.stdout).unwrap();
        assert_eq!(
            next["repair_brief"]["checker_id"], "rust.cargo_audit",
            "{name}: {next}"
        );
        evidence.push(serde_json::json!({"case":name,"report":report,"next":next}));
    }
    if let Ok(path) = std::env::var("CODEGUARD_TEST_CARGO_AUDIT_DATABASE_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
fn symlinked_collection_does_not_execute_selected_audit_tool() {
    let fixture = Fixture::new();
    let outside = fixture.0.join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, fixture.0.join("advisory-db/crates")).unwrap();
    let tool = fixture.0.join("audit-tool");
    fs::write(&tool, "#!/bin/sh\ntouch called\nprintf '{}\\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (_, report) = fixture.run(&tool);
    assert_eq!(
        report["reason"],
        "cargo_audit_database_snapshot_unavailable"
    );
    assert!(!fixture.0.join("called").exists());
}

#[test]
fn oversized_database_entry_is_an_environment_limit_without_tool_execution() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("advisory-db/crates/time")).unwrap();
    fs::write(
        fixture.0.join("advisory-db/crates/time/large.md"),
        vec![b'a'; 2 * 1024 * 1024 + 1],
    )
    .unwrap();
    let tool = fixture.0.join("audit-tool");
    fs::write(&tool, "#!/bin/sh\ntouch called\nprintf '{}\\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let (_, report) = fixture.run(&tool);
    assert_eq!(
        report["reason"],
        "cargo_audit_database_snapshot_limit_exceeded"
    );
    assert_eq!(report["findings"], serde_json::json!([]));
    assert!(!fixture.0.join("called").exists());
}
