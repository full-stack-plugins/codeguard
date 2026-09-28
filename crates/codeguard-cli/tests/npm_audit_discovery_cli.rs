use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn discovery_retains_per_project_audit_declarations_without_running_scripts() {
    let root = std::env::temp_dir().join(format!("cg-npm-discover-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    for (dir, raw) in [
        (
            "configured",
            r#"{"name":"a","version":"1.0.0","scripts":{"security":"npm audit --json"}}"#,
        ),
        ("missing", r#"{"name":"b","version":"1.0.0"}"#),
        (
            "custom",
            r#"{"name":"c","version":"1.0.0","scripts":{"audit":"npm audit && touch SHOULD_NOT_EXIST"}}"#,
        ),
        (
            "invalid",
            r#"{"name":"d","version":"1.0.0","scripts":true}"#,
        ),
    ] {
        fs::create_dir(fixture.0.join(dir)).unwrap();
        fs::write(fixture.0.join(dir).join("package.json"), raw).unwrap();
    }
    let result = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    let configs = value["checker_configurations"].as_array().unwrap();
    for (root, state) in [
        ("configured", "configured"),
        ("missing", "missing"),
        ("custom", "unknown"),
        ("invalid", "invalid"),
    ] {
        let entry = configs
            .iter()
            .find(|e| e["checker_id"] == "node.npm.audit" && e["build_root"] == root)
            .expect("per-root npm audit declaration");
        assert_eq!(entry["configuration"], state);
        assert_eq!(entry["category"], "cve");
        assert!(
            entry["next_action"]
                .as_str()
                .unwrap()
                .contains("漏洞数据时效")
        );
    }
    for language in ["all", "typescript"] {
        let result = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "plan",
                "cve",
                language,
                fixture.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(3));
        let plan: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(plan["quality_decision"], "not_evaluated");
        assert_eq!(
            plan["observed_checkers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["checker_id"] == "node.npm.audit")
                .count(),
            4
        );
        assert_eq!(
            plan["candidate_tasks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["checker_id"] == "node.npm.audit")
                .count(),
            1
        );
    }
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format",
            "human",
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("node.npm.audit"));
    assert!(human.contains("构建根：\"custom\""));
    assert!(human.contains("npm_audit_custom_invocation_requires_review"));
    assert!(human.contains("漏洞数据时效"));
    assert!(!fixture.0.join("codeguard").exists());
    assert!(!fixture.0.join("custom/SHOULD_NOT_EXIST").exists());
}
