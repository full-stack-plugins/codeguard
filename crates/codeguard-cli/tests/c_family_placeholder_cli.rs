use serde_json::Value;
use std::{fs, process::Command};

#[test]
#[ignore = "requires explicit existing Clang via CODEGUARD_CLANG_BIN"]
fn public_comments_exposes_same_scan_placeholder_policy_and_withdraws_on_failure() {
    check_language("c", "c11", "c");
    check_language("cpp", "c++17", "cpp");
}

fn check_language(language: &str, standard: &str, extension: &str) {
    let tool = std::env::var("CODEGUARD_CLANG_BIN").unwrap();
    let root = std::env::temp_dir().join(format!(
        "cg-placeholder-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let source = root.join(format!("api.{extension}"));
    fs::write(
        &source,
        "/// TODO.\n/// @param x TBD\n/// @return FIXME!\nint f(int x);\n",
    )
    .unwrap();
    let scan = |selected: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", language])
            .arg(&source)
            .args([
                "--clang-tool",
                selected,
                "--standard",
                standard,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let first = scan(&tool);
    assert_eq!(first["schema_version"], "0.10.0");
    assert_eq!(first["documentation_placeholders"]["status"], "observed");
    assert_eq!(
        first["documentation_placeholders"]["observation"]["positions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        first["documentation_placeholders"]["observation"]["authority"],
        "codeguard_structural_policy"
    );
    assert_eq!(first["placeholder_task_workflow_status"], "not_integrated");
    assert!(!root.join(".codeguard").exists());
    fs::write(&source,"/// Explain TODO queue processing.\n/// @param x The input value.\n/// @return The input unchanged.\nint f(int x);\n").unwrap();
    let repaired = scan(&tool);
    assert_eq!(
        repaired["documentation_placeholders"]["observation"]["positions"],
        serde_json::json!([])
    );
    assert_eq!(repaired["detailed_contract_qualification"], "not_granted");
    let missing = scan(root.join("missing-clang").to_str().unwrap());
    assert_eq!(
        missing["documentation_placeholders"]["status"],
        "incomplete"
    );
    assert!(missing["documentation_placeholders"]["observation"].is_null());
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&root)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::write(
        &source,
        "/// TODO.\n/// @param x TBD\n/// @return FIXME!\nint f(int x);\n",
    )
    .unwrap();
    let bound = scan(&tool);
    assert_eq!(bound["schema_version"], "0.11.0");
    assert_eq!(bound["placeholder_task_workflow_status"], "partial");
    assert_eq!(bound["placeholder_workbench"]["status"], "synced_partial");
    let ids = bound["placeholder_workbench"]["task_ids"]
        .as_array()
        .unwrap();
    assert_eq!(ids.len(), 1);
    let verify = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify", ids[0].as_str().unwrap()])
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let present_recheck = verify();
    assert_eq!(present_recheck["observation"], "still_present");
    assert_eq!(present_recheck["event_persisted"], true);
    let next = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("next")
            .arg(&root)
            .arg("--format=json")
            .output()
            .unwrap();
        assert!(matches!(output.status.code(), Some(0 | 3)));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let guidance = next();
    assert_eq!(guidance["schema_version"], "0.34.0");
    assert_eq!(guidance["repair_brief"]["task_id"], ids[0]);
    assert_eq!(
        guidance["repair_brief"]["placeholder_positions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        guidance["repair_brief"]["allowed_paths"],
        serde_json::json!([])
    );
    assert_eq!(
        guidance["repair_brief"]["attempt_history_status"],
        "not_integrated"
    );
    fs::write(&source, "int changed(int x);\n").unwrap();
    let stale_guidance = next();
    assert_eq!(
        stale_guidance["repair_brief"]["observation_status"],
        "input_changed"
    );
    assert_eq!(
        stale_guidance["repair_brief"]["placeholder_positions"],
        serde_json::json!([])
    );
    fs::write(
        &source,
        "/// TODO.\n/// @param x TBD\n/// @return FIXME!\nint f(int x);\n",
    )
    .unwrap();
    let first_run = bound["placeholder_workbench"]["run_id"].as_str().unwrap();
    let marker = root.join(format!(".codeguard/state/consumed/{first_run}.json"));
    let marker_bytes = fs::read(&marker).unwrap();
    fs::write(&marker, b"{}").unwrap();
    assert!(next()["repair_brief"].is_null());
    fs::write(&marker, marker_bytes).unwrap();
    let repeated = scan(&tool);
    assert_eq!(
        repeated["placeholder_workbench"]["task_ids"],
        bound["placeholder_workbench"]["task_ids"]
    );
    let run = bound["placeholder_workbench"]["run_id"].as_str().unwrap();
    assert!(
        root.join(format!(".codeguard/state/consumed/{run}.json"))
            .is_file()
    );
    fs::write(&source,"/// Return the input unchanged.\n/// @param x Input value.\n/// @return The same value.\nint f(int x);\n").unwrap();
    let clean = scan(&tool);
    assert_eq!(
        clean["placeholder_workbench"]["task_ids"],
        serde_json::json!([])
    );
    let absent_recheck = verify();
    assert_eq!(
        absent_recheck["observation"],
        "candidate_absent_unverified_policy"
    );
    assert_eq!(absent_recheck["event_persisted"], true);
    let fact: Value = serde_json::from_slice(
        &fs::read(root.join(format!(
            ".codeguard/findings/{}/finding.json",
            ids[0].as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    let blocked = scan(root.join("missing-clang").to_str().unwrap());
    assert_eq!(blocked["placeholder_workbench"]["status"], "incomplete");
    assert_eq!(
        blocked["placeholder_workbench"]["task_ids"],
        serde_json::json!([])
    );
    if let Ok(destination) = std::env::var("CODEGUARD_PLACEHOLDER_RECHECK_EVIDENCE") {
        assert!(std::path::Path::new(&destination).is_absolute());
        fs::write(
            format!("{destination}.{language}.json"),
            serde_json::to_vec_pretty(&serde_json::json!([present_recheck, absent_recheck]))
                .unwrap(),
        )
        .unwrap();
    }
    if let Ok(destination) = std::env::var("CODEGUARD_PLACEHOLDER_NEXT_EVIDENCE") {
        assert!(std::path::Path::new(&destination).is_absolute());
        fs::write(
            destination,
            serde_json::to_vec_pretty(&serde_json::json!([guidance, stale_guidance])).unwrap(),
        )
        .unwrap();
    }
    if let Ok(destination) = std::env::var("CODEGUARD_PLACEHOLDER_CLI_EVIDENCE") {
        assert!(std::path::Path::new(&destination).is_absolute());
        fs::write(
            destination,
            serde_json::to_vec_pretty(&serde_json::json!([
                first, repaired, missing, bound, repeated, clean, blocked
            ]))
            .unwrap(),
        )
        .unwrap();
    }
}
