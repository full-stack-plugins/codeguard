#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn aggregate_keeps_python_structure_and_reuses_lint_confirmation_task() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-check-python-structure-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("app.py"), "def run():\n").unwrap();
    let command = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let path = root.to_str().unwrap();
    command(&["init", path, "--apply", "--format=json"]);
    let lint = command(&["lint", "python", path, "--file", "app.py", "--format=json"]);
    let id = lint["setup"]["task_id"].as_str().unwrap();
    for selection in ["python", "all"] {
        let report = command(&["check", selection, path, "--format=json"]);
        assert_eq!(report["schema_version"], "0.48.0");
        let row = report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == "app.py")
            .unwrap();
        assert_eq!(row["recovery_count"], 0);
        assert_eq!(row["recoveries"], serde_json::json!([]));
        assert_eq!(row["structural_observation_count"], 1);
        assert_eq!(
            row["structural_observations"][0]["rule_id"],
            "codeguard.python.required_suite"
        );
        assert!(
            report["syntax_tasks"]["tasks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|task| task["task_id"] == id),
            "{report}"
        );
    }
    fs::write(root.join("app.py"), "def run():\n    pass\n").unwrap();
    let clean = command(&["check", "python", path, "--format=json"]);
    assert_ne!(clean["schema_version"], "0.48.0");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            root.join(".codeguard/findings")
                .join(id)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::remove_dir_all(root).unwrap();
}
