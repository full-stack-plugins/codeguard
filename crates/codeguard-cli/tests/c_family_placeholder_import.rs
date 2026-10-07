use serde_json::{Value, json};
use std::{fs, process::Command, path::Path};

#[test]
#[ignore = "requires explicit existing Clang via CODEGUARD_CLANG_BIN"]
fn placeholder_import_reuses_file_identity_and_never_closes_on_clean_observation() {
    let tool = std::env::var("CODEGUARD_CLANG_BIN").unwrap();
    let root = std::env::temp_dir().join(format!(
        "cg-placeholder-import-{}-{}",
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
    let source = root.join("api.c");
    let cli = |args: Vec<String>| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg("--format=json")
            .output()
            .unwrap();
        assert!(
            matches!(output.status.code(), Some(0 | 3)),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    cli(vec![
        "init".into(),
        root.display().to_string(),
        "--apply".into(),
    ]);
    let scan = || {
        cli(vec![
            "comments".into(),
            "c".into(),
            source.display().to_string(),
            "--clang-tool".into(),
            tool.clone(),
            "--standard".into(),
            "c11".into(),
        ])
    };
    let import = |feedback: &Value, index: usize| {
        let run = format!("clangdocplaceholder-1-1-{index}");
        let packet = json!({"schema_version":"0.1.0","report_type":"clang_documentation_placeholder_workbench_observation","workspace_binding":"bound","workspace_id":feedback["workbench"]["workspace_id"],"run_id":run,"path":"api.c","language":"c","standard":"c11","profile":"clang-documentation-placeholder-v1","selected_tool":tool,"source_sha256":feedback["source_sha256"],"local_scan_complete":feedback["local_scan_complete"],"native":feedback["native"],"structure":feedback["documentation_structure"],"placeholders":feedback["documentation_placeholders"]["observation"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
        fs::write(
            root.join(format!(".codeguard/reports/{run}.json")),
            serde_json::to_vec_pretty(&packet).unwrap(),
        )
        .unwrap();
        cli(vec![
            "work".into(),
            "sync".into(),
            root.display().to_string(),
        ]);
        assert!(
            root.join(format!(".codeguard/state/consumed/{run}.json"))
                .is_file()
        );
    };
    let facts = || {
        fs::read_dir(root.join(".codeguard/findings"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>()
    };
    let initial = "/// TODO.\n/// @param x TBD\n/// @return FIXME!\nint f(int x);\n";
    fs::write(&source, initial).unwrap();
    import(&scan(), 1);
    let first = facts();
    assert_eq!(first.len(), 1);
    let fact_path = first[0].join("finding.json");
    let fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    assert_eq!(fact["checker_id"], "c.clang.documentation_placeholder");
    assert_eq!(
        fact["native_rule_id"],
        "codeguard.documentation.placeholder_description"
    );
    fs::write(&source, format!("\n{initial}")).unwrap();
    import(&scan(), 2);
    assert_eq!(facts(), first);
    fs::write(&source,"/// Return the input unchanged.\n/// @param x Input value.\n/// @return The same value.\nint f(int x);\n").unwrap();
    import(&scan(), 3);
    assert_eq!(facts(), first);
    let preserved: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    assert_eq!(preserved["state"], "open");
    assert!(Path::new(&fact_path).is_file());
    let mut forged: Value = serde_json::from_slice(
        &fs::read(root.join(".codeguard/reports/clangdocplaceholder-1-1-3.json")).unwrap(),
    )
    .unwrap();
    forged["run_id"] = json!("clangdocplaceholder-1-1-4");
    let row = &forged["structure"]["observation"]["functions"][0];
    let bad_position = json!({"offset_byte":row["offset_byte"],"line":row["line"],"column_byte":row["column_byte"],"component":"parameter:foreign"});
    forged["placeholders"]["positions"] = json!([bad_position]);
    fs::write(
        root.join(".codeguard/reports/clangdocplaceholder-1-1-4.json"),
        serde_json::to_vec_pretty(&forged).unwrap(),
    )
    .unwrap();
    cli(vec![
        "work".into(),
        "sync".into(),
        root.display().to_string(),
    ]);
    assert!(
        !root
            .join(".codeguard/state/consumed/clangdocplaceholder-1-1-4.json")
            .exists()
    );
    assert_eq!(facts(), first);
    let task_id = first[0].file_name().unwrap().to_str().unwrap();
    let verification = cli(vec![
        "task".into(),
        "verify".into(),
        task_id.into(),
        root.display().to_string(),
    ]);
    assert_eq!(
        verification["reason"],
        "clang_placeholder_task_workflow_not_integrated"
    );
    assert!(verification["native_scan"].is_null());
    assert_eq!(verification["event_persisted"], false);
}
