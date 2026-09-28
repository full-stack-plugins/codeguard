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
fn profile_refresh_cannot_drop_a_recorded_npm_preparation_scope() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-recorded-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let root = &fixture.0;
    let project = root.join("a");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    fs::write(
        project.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let path = root.to_str().unwrap();
    run(&["init", path, "--apply", "--format", "json"]);
    run(&["check", "all", path, "--format", "json"]);
    let facts = root.join("codeguard/findings");
    let fact_dir = fs::read_dir(&facts)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let id = fact_dir.file_name().unwrap().to_str().unwrap();
    let fact_path = fact_dir.join("finding.json");
    let original = fs::read(&fact_path).unwrap();
    fs::remove_file(project.join("package.json")).unwrap();
    run(&["init", path, "--apply", "--format", "json"]);
    let profile: Value =
        serde_json::from_slice(&fs::read(root.join("codeguard/project.json")).unwrap()).unwrap();
    assert!(profile["manifest_sha256"].get("a/package.json").is_none());
    for _ in 0..2 {
        let report = run(&["check", "all", path, "--format", "json"]);
        let scans = report["native_results"]["npm_cve"].as_array().unwrap();
        assert_eq!(scans.len(), 1, "{report}");
        assert_eq!(scans[0]["build_root"], "a");
        assert_eq!(scans[0]["feedback"]["reason"], "npm_manifest_missing");
        assert_eq!(scans[0]["feedback"]["workbench"]["new_blockers"], 0);
        assert!(scans[0]["observation"]["manifest_sha256"].is_null());
        assert_eq!(
            fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
            1
        );
    }
    // 可编辑任务文本不控制检查范围；删除附件不能消除事实记录。
    let task = root.join("codeguard/tasks").join(format!("{id}.md"));
    let task_bytes = fs::read(&task).unwrap();
    fs::remove_file(&task).unwrap();
    let report = run(&["check", "all", path, "--format", "json"]);
    assert_eq!(
        report["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::write(&task, task_bytes).unwrap();
    let sibling = root.join("ab");
    fs::create_dir(&sibling).unwrap();
    fs::write(
        sibling.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    fs::write(
        sibling.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    for field in [
        "workspace_id",
        "fingerprint",
        "build_root",
        "schema_version",
        "scope",
        "state",
        "authority",
        "first_report_sha256",
        "first_run_id",
    ] {
        let mut bad: Value = serde_json::from_slice(&original).unwrap();
        bad[field] = serde_json::json!(if field == "build_root" {
            "../outside"
        } else if field == "first_run_id" {
            "invalid/run"
        } else {
            "invalid"
        });
        fs::write(&fact_path, bad.to_string()).unwrap();
        let report = run(&["check", "all", path, "--format", "json"]);
        assert!(
            report["unresolved_conditions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "recorded_npm_scope_unavailable"),
            "{report}"
        );
        let scans = report["native_results"]["npm_cve"].as_array().unwrap();
        assert_eq!(scans.len(), 1, "{report}");
        assert_eq!(scans[0]["build_root"], "ab");
    }
    fs::write(&fact_path, original).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    let report = run(&["check", "all", path, "--format", "json"]);
    assert_eq!(
        report["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
        2
    );
}
