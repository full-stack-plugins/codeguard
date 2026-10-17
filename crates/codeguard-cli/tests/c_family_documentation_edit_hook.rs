#![cfg(unix)]
//! 编辑快检必须保留显式文档上下文及原生语法覆盖缺口。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn selected_c_file_without_standard_reports_documentation_context_gap() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-c-edit-context-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.c"), "int f(int x) { return x; }\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["hook", "execute"])
        .arg(&root)
        .args(["--timeout", "5s", "--format=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":["a.c"],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["local_feedback"]["c_family_documentation"]["c"]["reason"],
        "clang_documentation_context_required"
    );
    assert_eq!(
        report["local_feedback"]["c_family_documentation"]["c"]["native_task_started"],
        false
    );
    assert_eq!(report["local_feedback"]["coverage_proven"], false);
    assert!(!root.join(".codeguard").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing Apple Clang via CODEGUARD_CLANG_BIN"]
fn confirmed_c_and_cpp_edits_run_explicit_native_documentation_without_claiming_syntax_coverage() {
    let tool = std::env::var("CODEGUARD_CLANG_BIN").expect("explicit existing Clang");
    for (language, extension, option, standard) in [
        ("c", "c", "--c-standard", "c11"),
        ("cpp", "cpp", "--cpp-standard", "c++17"),
    ] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-{}-edit-native-{}",
            language,
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let relative = format!("a.{extension}");
        let source = "/** Compute value.\n * @param x\n */\nint f(int x) { return x; }\n";
        fs::write(root.join(&relative), source).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["hook", "execute"])
            .arg(&root)
            .args([
                "--timeout",
                "15s",
                "--format=json",
                "--clang-tool",
                &tool,
                option,
                standard,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":[relative],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(3), "{out:?}");
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.31.0");
        let scan = &report["local_feedback"]["c_family_documentation"][language];
        assert_eq!(scan["native_task_started"], true, "{report}");
        assert_eq!(scan["standard"], standard);
        assert_eq!(scan["source_file_count"], 1);
        assert_eq!(scan["files"][0]["current"], true, "{report}");
        assert!(
            scan["files"][0]["feedback"]["native"]["diagnostics"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "{report}"
        );
        assert_eq!(report["local_feedback"]["coverage_proven"], false);
        assert_eq!(fs::read_to_string(root.join(relative)).unwrap(), source);
        assert!(!root.join(".codeguard").exists());
        let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap();
        assert_eq!(initialized.status.code(), Some(3), "{initialized:?}");
        assert!(root.join(".codeguard/workspace.json").is_file());
        let mut stable_ids = Vec::new();
        for attempt in 0..2 {
            let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["hook", "execute"])
                .arg(&root)
                .args([
                    "--timeout",
                    "15s",
                    "--format=json",
                    "--clang-tool",
                    &tool,
                    option,
                    standard,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(json!({"schema_version":"1.0.0","report_type":"hook_trigger_request","input":{"event":"file_changed","changed_paths":[format!("a.{extension}")],"task_id":null,"write_outcome":"confirmed","host_claims_blocking":false}}).to_string().as_bytes()).unwrap();
            let out = child.wait_with_output().unwrap();
            assert_eq!(out.status.code(), Some(3), "{out:?}");
            let feedback: Value = serde_json::from_slice(&out.stdout).unwrap();
            let native = &feedback["local_feedback"]["c_family_documentation"][language]["files"]
                [0]["feedback"];
            let ids = vec![
                native["workbench"]["task_ids"].clone(),
                native["structural_workbench"]["task_ids"].clone(),
            ];
            assert!(
                ids.iter()
                    .all(|ids| ids.as_array().is_some_and(|a| !a.is_empty())),
                "{feedback}"
            );
            if attempt == 0 {
                stable_ids = ids;
            } else {
                assert_eq!(ids, stable_ids);
            }
        }
        if let Ok(directory) = std::env::var("CODEGUARD_C_EDIT_EVIDENCE_DIR") {
            let directory = std::path::Path::new(&directory);
            assert!(directory.is_absolute());
            fs::write(directory.join(format!("c-family-edit-{}-{language}.json", if cfg!(feature="wasm-precheck") {"wasm"} else {"default"})),serde_json::to_vec_pretty(&json!({"qualification":"not_granted","test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_documentation_edit_hook.rs"))),"cli_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"stable_task_ids":stable_ids,"report":report})).unwrap()).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
