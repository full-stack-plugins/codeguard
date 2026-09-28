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
fn unavailable_inputs_produce_stable_tasks_and_verification_history_without_fake_digests() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-input-state-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let root = &fixture.0;
    let manifest = root.join("package.json");
    let lock = root.join("package-lock.json");
    let valid = r#"{"scripts":{"audit":"npm audit"}}"#;
    fs::write(&manifest, valid).unwrap();
    fs::write(&lock, r#"{"lockfileVersion":3,"packages":{"":{}}}"#).unwrap();
    let run = |args: &[&str]| {
        let p = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        (
            p.status.code(),
            serde_json::from_slice::<Value>(&p.stdout).unwrap(),
        )
    };
    let path = root.to_str().unwrap();
    run(&["init", path, "--apply", "--format", "json"]);
    let mut last_verify = Value::Null;
    for state in ["missing", "not_regular", "path_alias", "unavailable"] {
        fs::remove_file(&manifest).unwrap();
        match state {
            "missing" => {}
            "not_regular" => fs::create_dir(&manifest).unwrap(),
            "path_alias" => std::os::unix::fs::symlink(root.join("absent"), &manifest).unwrap(),
            _ => fs::write(&manifest, vec![b' '; 256 * 1024 + 1]).unwrap(),
        }
        let (code, report) = run(&["cve", "typescript", path, "--format", "json"]);
        assert_eq!(code, Some(3));
        assert_eq!(
            report["reason"],
            format!("npm_manifest_{state}"),
            "{report}"
        );
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        let (_, explicit) = run(&[
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
        assert_eq!(explicit["reason"], format!("npm_manifest_{state}"));
        assert_eq!(explicit["local_coherent"], false);

        let (_, next) = run(&["next", path, "--format", "json"]);
        let id = next["repair_brief"]["task_id"]
            .as_str()
            .unwrap_or_else(|| panic!("state={state}, next={next}"));
        let (_, verify) = run(&["task", "verify", id, path, "--format", "json"]);
        assert_eq!(verify["event_persisted"], true, "{verify}");
        assert_eq!(verify["observation"], "incomplete");
        assert_eq!(verify["native_scan"]["manifest_state"], state);
        assert!(verify["native_scan"]["manifest_sha256"].is_null());
        last_verify = verify["native_scan"].clone();
        assert_eq!(
            fs::read_dir(root.join(".codeguard/tasks")).unwrap().count(),
            1
        );
        if state == "not_regular" {
            fs::remove_dir(&manifest).unwrap();
        } else if state != "missing" {
            fs::remove_file(&manifest).unwrap();
        }
        fs::write(&manifest, valid).unwrap();
    }
    let (_, restored) = run(&["cve", "typescript", path, "--format", "json"]);
    assert_eq!(restored["reason"], "npm_execution_context_missing");
    assert_eq!(restored["workbench"]["new_blockers"], 0);
    let (code, next) = run(&["next", path, "--format", "json"]);
    assert_eq!(code, Some(0), "{next}");
    for (i, (key, value)) in [
        ("manifest_sha256", serde_json::json!("0".repeat(64))),
        ("manifest_state", serde_json::json!("unknown")),
        (
            "diagnostic_reason",
            serde_json::json!("npm_lock_not_regular"),
        ),
        ("coverage_proven", serde_json::json!(true)),
        ("component_count", serde_json::json!(1)),
        ("workspace_id", serde_json::json!("wrong-workspace")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut bad = last_verify.clone();
        bad["run_id"] = serde_json::json!(format!("npm-bad-input-{i}"));
        bad[key] = value;
        fs::write(
            root.join(format!(".codeguard/reports/npm-bad-input-{i}.json")),
            bad.to_string(),
        )
        .unwrap();
    }
    last_verify["run_id"] = serde_json::json!("npm-stale-input");
    fs::write(
        root.join(".codeguard/reports/npm-stale-input.json"),
        last_verify.to_string(),
    )
    .unwrap();
    let (_, synced) = run(&["work", "sync", path, "--format", "json"]);
    assert_eq!(synced["failed_reports"], 7, "{synced}");
    assert_eq!(
        fs::read_dir(root.join(".codeguard/tasks")).unwrap().count(),
        1
    );
    assert!(!root.join("node_modules").exists());
}
