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
fn npm_cve_budget_uses_cli_environment_project_and_builtin_in_order() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-budget-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("package.json"), "{}").unwrap();
    fs::create_dir(fixture.0.join("codeguard")).unwrap();
    let config = fixture.0.join("codeguard/runtime.json");
    let run = |extra: &[&str], env: Option<&str>| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.args([
            "cve",
            "typescript",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .args(extra)
        .env_remove("CODEGUARD_TIMEOUT");
        if let Some(v) = env {
            cmd.env("CODEGUARD_TIMEOUT", v);
        }
        cmd.output().unwrap()
    };
    for (raw, extra, env, expected, source) in [
        (None, vec![], None, 1800000, "builtin_default"),
        (
            Some(
                r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s"}"#,
            ),
            vec![],
            None,
            2000,
            "project_default",
        ),
        (
            Some(
                r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s"}"#,
            ),
            vec![],
            Some("3s"),
            3000,
            "registered_environment",
        ),
        (
            Some(
                r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s"}"#,
            ),
            vec!["--timeout", "4s"],
            Some("bad"),
            4000,
            "cli",
        ),
    ] {
        if let Some(raw) = raw {
            fs::write(&config, raw).unwrap();
        }
        let out = run(&extra, env);
        assert_eq!(out.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["execution_budget"]["timeout_ms"], expected);
        assert_eq!(report["execution_budget"]["source"], source);
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
    fs::write(&config,r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"2s","exclude":["src"]}"#).unwrap();
    let invalid = run(&[], None);
    assert_eq!(invalid.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(report["reason"], "npm_runtime_defaults_invalid");
    assert!(report["execution_budget"].is_null());
    assert_eq!(run(&[], Some("bad")).status.code(), Some(2));
    let overridden = run(&["--timeout", "4s"], None);
    let report: Value = serde_json::from_slice(&overridden.stdout).unwrap();
    assert_eq!(report["execution_budget"]["source"], "cli");
    use std::os::unix::fs::{PermissionsExt, symlink};
    let node = fixture.0.join("node");
    fs::write(&node, "#!/bin/sh\nprintf ran > SHOULD_NOT_EXIST\n").unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    for file in ["npm.js", "user.npmrc", "global.npmrc"] {
        fs::write(fixture.0.join(file), "").unwrap();
    }
    fs::write(
        fixture.0.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let paths: Vec<_> = ["node", "npm.js", "user.npmrc", "global.npmrc"]
        .iter()
        .map(|f| fixture.0.join(f))
        .collect();
    let out = run(
        &[
            "--node-tool",
            paths[0].to_str().unwrap(),
            "--npm-entry",
            paths[1].to_str().unwrap(),
            "--npm-version",
            "11.16.0",
            "--userconfig",
            paths[2].to_str().unwrap(),
            "--globalconfig",
            paths[3].to_str().unwrap(),
        ],
        None,
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["reason"], "npm_runtime_defaults_invalid");
    assert!(!fixture.0.join("SHOULD_NOT_EXIST").exists());
    fs::remove_file(&config).unwrap();
    symlink(fixture.0.join("missing-config"), &config).unwrap();
    let out = run(&[], None);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["reason"], "npm_runtime_defaults_invalid");
}
