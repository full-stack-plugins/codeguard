#![cfg(all(feature = "wasm-precheck", unix))]

use serde_json::{Value, json};
use std::{fs, process::Command};

fn command(args: &[&str], request: Option<Value>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command.args(args).env("PATH", "");
    let out = if let Some(request) = request {
        use std::io::Write;
        use std::process::Stdio;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(request.to_string().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.output().unwrap()
    };
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn public_go_package_candidate_reuses_one_task_across_checks_and_save() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-go-package-public-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("go.mod"),
        "module example.test/sample\n\ngo 1.23\n",
    )
    .unwrap();
    fs::write(root.join("main.go"), "// package fake\nfunc f() {}\n").unwrap();
    let path = root.to_str().unwrap();
    command(&["init", path, "--apply", "--format=json"], None);
    let source = root.join("main.go");
    let probe = command(
        &[
            "grammar",
            "probe",
            "go",
            source.to_str().unwrap(),
            "--format=json",
        ],
        None,
    );
    assert_eq!(probe["schema_version"], "0.3.0");
    assert_eq!(probe["source_scope"], "whole_file");
    assert_eq!(probe["recoveries"], json!([]));
    assert_eq!(
        probe["structural_observations"][0]["rule_id"],
        "codeguard.go.required_package"
    );
    assert_eq!(probe["native"]["status"], "not_run");
    let mut task = None;
    for selection in ["go", "all", "go"] {
        let report = command(&["check", selection, path, "--format=json"], None);
        assert_eq!(
            report["schema_version"],
            if report["next"]["schema_version"] == "0.14.0" {
                "0.50.0"
            } else {
                "0.49.0"
            }
        );
        let row = report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == "main.go")
            .unwrap();
        assert_eq!(row["recovery_count"], 0);
        assert_eq!(row["structural_observation_count"], 1);
        let found = report["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == "main.go")
            .unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        if let Some(expected) = &task {
            assert_eq!(&found, expected);
        } else {
            task = Some(found);
        }
    }
    let task = task.unwrap();
    let hook = command(
        &["hook", "execute", path, "--timeout=30s", "--format=json"],
        Some(
            json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["main.go"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}),
        ),
    );
    assert_eq!(
        hook["local_feedback"]["next_action"],
        "require_native_lint_confirmation"
    );
    assert_eq!(hook["local_feedback"]["candidate_recovery_count"], 0);
    assert_eq!(hook["local_feedback"]["candidate_structure_count"], 1);
    assert_eq!(
        hook["local_feedback"]["syntax_tasks"]["tasks"][0]["task_id"],
        task
    );
    let text =
        fs::read_to_string(root.join(".codeguard/tasks").join(format!("{task}.md"))).unwrap();
    assert!(text.contains("package"), "{text}");
    assert!(text.contains("原生"));
    let initial = codeguard_cli::next_command::read_task_brief(&root, &task).unwrap();
    assert_eq!(initial["schema_version"], "0.14.0", "{initial}");
    assert!(initial["step"].as_str().unwrap().contains("Go"));
    assert!(!initial["step"].as_str().unwrap().contains("尚未接入"));
    let unavailable = command(&["task", "verify", &task, path, "--format=json"], None);
    assert_eq!(unavailable["event_persisted"], true, "{unavailable}");
    assert_eq!(
        unavailable["native_scan"]["native"]["reason"],
        "go_syntax_tool_not_provided"
    );
    let mismatched = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            &task,
            path,
            "--zig-tool",
            "/missing/zig",
            "--format=json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(mismatched.status.code(), Some(2));
    let fact_path = root.join(format!(".codeguard/findings/{task}/finding.json"));
    let fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    let receipt = root.join(format!(
        ".codeguard/state/consumed/{}.json",
        fact["first_run_id"].as_str().unwrap()
    ));
    let original_receipt = fs::read(&receipt).unwrap();
    fs::write(&receipt, b"{}").unwrap();
    let damaged = command(&["task", "verify", &task, path, "--format=json"], None);
    assert_eq!(damaged["event_persisted"], false);
    assert_eq!(damaged["reason"], "syntax_original_report_not_synced");
    fs::write(&receipt, original_receipt).unwrap();
    fs::write(&source, "package main\nfunc f() {}\n").unwrap();
    let fixed = command(&["check", "go", path, "--format=json"], None);
    let row = fixed["syntax_candidates"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == "main.go")
        .unwrap();
    assert!(row.get("structural_observations").is_none());
    let fact: Value = serde_json::from_slice(
        &fs::read(
            root.join(".codeguard/findings")
                .join(&task)
                .join("finding.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires existing Go 1.23.4 SDK via CODEGUARD_GO_TOOL"]
fn actual_go_confirms_package_task_before_and_after_repair() {
    let tool = std::env::var("CODEGUARD_GO_TOOL").unwrap();
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-go-package-native-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("go.mod"),
        "module example.test/sample\n\ngo 1.23\n",
    )
    .unwrap();
    fs::write(root.join("main.go"), "func f() {}\n").unwrap();
    let path = root.to_str().unwrap();
    command(&["init", path, "--apply", "--format=json"], None);
    let checked = command(&["check", "go", path, "--format=json"], None);
    let task = checked["syntax_tasks"]["tasks"][0]["task_id"]
        .as_str()
        .unwrap();
    for (source, observation) in [
        ("func f() {}\n", "still_blocked"),
        (
            "package main\nfunc f() {}\n",
            "candidate_absent_unverified_policy",
        ),
    ] {
        fs::write(root.join("main.go"), source).unwrap();
        let report = command(
            &[
                "task",
                "verify",
                task,
                path,
                "--go-tool",
                &tool,
                "--format=json",
            ],
            None,
        );
        assert_eq!(report["schema_version"], "0.22.0", "{report}");
        assert_eq!(report["native_scan"]["schema_version"], "0.9.0");
        assert_eq!(report["observation"], observation, "{report}");
        assert_eq!(report["event_persisted"], true, "{report}");
        assert_eq!(report["native_scan"]["input_stable"], true);
        let brief = codeguard_cli::next_command::read_task_brief(&root, task).unwrap();
        assert_eq!(brief["schema_version"], "0.14.0");
        assert_eq!(brief["recheck_argv"][7], "--go-tool");
        assert!(!brief["step"].as_str().unwrap().contains("Zig"));
        if observation == "still_blocked" {
            assert!(brief["step"].as_str().unwrap().contains("Go"));
        }
        if let Ok(directory) = std::env::var("CODEGUARD_GO_RECHECK_EVIDENCE") {
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                std::path::Path::new(&directory).join(format!("brief-{observation}.json")),
                serde_json::to_vec_pretty(&brief).unwrap(),
            )
            .unwrap();
        }
        let fact: Value = serde_json::from_slice(
            &fs::read(root.join(format!(".codeguard/findings/{task}/finding.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        if let Ok(directory) = std::env::var("CODEGUARD_GO_RECHECK_EVIDENCE") {
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                std::path::Path::new(&directory).join(format!("{observation}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
    }
    fs::remove_dir_all(root).unwrap();
}
