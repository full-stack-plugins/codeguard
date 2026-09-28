#![cfg(unix)]
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn missing_lock_is_a_stable_preparation_task_and_changes_invalidate_its_receipt() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-no-lock-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(
        fixture.0.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code(), value)
    };
    let path = fixture.0.to_str().unwrap();
    run(&["init", path, "--apply", "--format", "json"]);
    for count in [1, 0] {
        let (code, report) = run(&["cve", "typescript", path, "--format", "json"]);
        assert_eq!(code, Some(3));
        assert_eq!(report["reason"], "npm_lock_missing", "{report}");
        assert_eq!(report["workbench_status"], "synced_partial");
        assert_eq!(report["workbench"]["new_blockers"], count);
    }
    // 完整显式上下文也不能绕过缺锁前置，且check all归属同一义务。
    let (_, checked) = run(&[
        "check",
        "all",
        path,
        "--node-tool",
        "/unused-node",
        "--format",
        "json",
    ]);
    assert_eq!(
        checked["native_results"]["npm_cve"][0]["feedback"]["reason"],
        "npm_lock_missing"
    );
    assert_eq!(
        checked["native_results"]["npm_cve"][0]["observation"]["lock_state"],
        "missing"
    );
    let (_, full) = run(&[
        "cve",
        "typescript",
        path,
        "--node-tool",
        "/unused-node",
        "--npm-entry",
        "/unused-entry",
        "--npm-version",
        "11.16.0",
        "--userconfig",
        "/unused-user",
        "--globalconfig",
        "/unused-global",
        "--format",
        "json",
    ]);
    assert_eq!(full["reason"], "npm_lock_missing");
    assert_eq!(full["local_coherent"], false);
    let (_, next) = run(&["next", path, "--format", "json"]);
    let id = next["repair_brief"]["task_id"].as_str().unwrap();
    let (_, verify) = run(&["task", "verify", id, path, "--format", "json"]);
    assert_eq!(verify["event_persisted"], true, "{verify}");
    assert_eq!(verify["observation"], "incomplete");
    assert_eq!(verify["native_scan"]["lock_state"], "missing");
    assert!(verify["native_scan"]["lock_sha256"].is_null());
    // 悬空符号链接不等同于缺锁，不能制造新的缺失证据。
    std::os::unix::fs::symlink(
        fixture.0.join("absent"),
        fixture.0.join("package-lock.json"),
    )
    .unwrap();
    let (_, alias) = run(&["cve", "typescript", path, "--format", "json"]);
    assert_eq!(alias["reason"], "npm_lock_path_alias");
    assert_eq!(alias["workbench_status"], "synced_partial");
    fs::remove_file(fixture.0.join("package-lock.json")).unwrap();
    fs::write(
        fixture.0.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let (code, report) = run(&["cve", "typescript", path, "--format", "json"]);
    assert_eq!(code, Some(3));
    assert_eq!(report["reason"], "npm_execution_context_missing");
    assert_eq!(report["workbench"]["new_blockers"], 0);
    assert_eq!(
        fs::read_dir(fixture.0.join("codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    let (code, next) = run(&["next", path, "--format", "json"]);
    assert_eq!(code, Some(0), "{next}");
    assert_eq!(next["repair_brief"]["task_id"], id);
    // 已恢复锁文件时不能重放缺锁观察，坏字段也不增任务。
    for (i, (key, value)) in [
        ("lock_state", serde_json::json!("present")),
        ("lock_sha256", serde_json::json!("0".repeat(64))),
        ("schema_version", serde_json::json!("0.3.0")),
        ("coverage_proven", serde_json::json!(true)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut forged = verify["native_scan"].clone();
        forged["run_id"] = serde_json::json!(format!("npm-forged-{i}"));
        forged[key] = value;
        fs::write(
            fixture
                .0
                .join(format!("codeguard/reports/npm-forged-{i}.json")),
            forged.to_string(),
        )
        .unwrap();
    }
    let mut stale = verify["native_scan"].clone();
    stale["run_id"] = serde_json::json!("npm-stale-missing-lock");
    fs::write(
        fixture
            .0
            .join("codeguard/reports/npm-stale-missing-lock.json"),
        stale.to_string(),
    )
    .unwrap();
    let (_, synced) = run(&["work", "sync", path, "--format", "json"]);
    assert_eq!(synced["failed_reports"], 5, "{synced}");
    assert_eq!(
        fs::read_dir(fixture.0.join("codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    assert!(!fixture.0.join("node_modules").exists());
}
