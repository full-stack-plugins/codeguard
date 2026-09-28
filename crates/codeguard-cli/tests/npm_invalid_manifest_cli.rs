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
fn malformed_manifest_stops_native_launch_and_enters_the_same_repair_task() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-invalid-manifest-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let root = &fixture.0;
    fs::write(root.join("package.json"), "{").unwrap();
    fs::write(
        root.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let node = root.join("node");
    let marker = root.join("launched");
    fs::write(
        &node,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    let entry = root.join("npm.js");
    let user = root.join("user.npmrc");
    let global = root.join("global.npmrc");
    for p in [&entry, &user, &global] {
        fs::write(p, "").unwrap();
    }
    let execute = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        (
            out.status.code(),
            serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        )
    };
    execute(&[
        "init",
        root.to_str().unwrap(),
        "--apply",
        "--format",
        "json",
    ]);
    let options = [
        "--node-tool",
        node.to_str().unwrap(),
        "--npm-entry",
        entry.to_str().unwrap(),
        "--npm-version",
        "11.16.0",
        "--userconfig",
        user.to_str().unwrap(),
        "--globalconfig",
        global.to_str().unwrap(),
        "--format",
        "json",
    ];
    for (input, reason) in [
        ("{", "npm_manifest_invalid"),
        (
            r#"{"scripts":{"audit":"npm audit"},"scripts":{}}"#,
            "npm_manifest_invalid",
        ),
        (r#"{"scripts":true}"#, "npm_scripts_invalid"),
    ] {
        fs::write(root.join("package.json"), input).unwrap();
        let mut argv = vec!["cve", "typescript", root.to_str().unwrap()];
        argv.extend(options);
        let (code, report) = execute(&argv);
        assert_eq!(code, Some(3));
        assert_eq!(report["reason"], reason, "{report}");
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        let (_, checked) = execute(&[
            "check",
            "all",
            root.to_str().unwrap(),
            "--node-tool",
            node.to_str().unwrap(),
            "--format",
            "json",
        ]);
        assert_eq!(
            checked["native_results"]["npm_cve"][0]["feedback"]["reason"], reason,
            "{checked}"
        );
        assert_eq!(report["local_coherent"], false);
        assert!(!marker.exists());
        assert_eq!(
            fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
            1
        );
    }
    let (_, next) = execute(&["next", root.to_str().unwrap(), "--format", "json"]);
    assert!(
        next["repair_brief"]["step"]
            .as_str()
            .unwrap()
            .contains("package.json")
    );
    let id = next["repair_brief"]["task_id"].as_str().unwrap();
    let mut argv = vec!["task", "verify", id, root.to_str().unwrap()];
    argv.extend(options);
    let (_, verified) = execute(&argv);
    assert_eq!(verified["event_persisted"], true, "{verified}");
    assert_eq!(verified["observation"], "incomplete");
    assert!(!marker.exists());
    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    let (_, report) = execute(&[
        "cve",
        "typescript",
        root.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(report["reason"], "npm_execution_context_missing");
    assert_eq!(report["workbench"]["new_blockers"], 0);
    let (code, next) = execute(&["next", root.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, Some(0), "{next}");
    assert_eq!(next["repair_brief"]["task_id"], id);
    // 即使伪造当前清单摘要，也不能在有效清单上声称配置损坏。
    use sha2::{Digest, Sha256};
    let mut forged = verified["native_scan"].clone();
    forged["run_id"] = serde_json::json!("npm-fake-manifest-invalid");
    forged["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(fs::read(root.join("package.json")).unwrap())
    ));
    fs::write(
        root.join("codeguard/reports/npm-fake-manifest-invalid.json"),
        forged.to_string(),
    )
    .unwrap();
    let (_, synced) = execute(&["work", "sync", root.to_str().unwrap(), "--format", "json"]);
    assert_eq!(synced["failed_reports"], 1, "{synced}");
    assert_eq!(
        fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
        1
    );
}
