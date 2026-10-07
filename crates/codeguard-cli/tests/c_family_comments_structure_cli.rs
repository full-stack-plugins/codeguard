use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};
use sha2::{Digest, Sha256};

#[test]
#[ignore = "requires installed fixed Apple Clang21; set CODEGUARD_CLANG_BIN"]
fn public_comments_exposes_absent_empty_and_valid_structural_descriptions() {
    let clang =
        PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("explicit existing tool"));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-structure-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let cases = [
        (
            "absent",
            "int f(int x) { return x; }",
            vec!["documentation_comment"],
        ),
        (
            "empty",
            "/** */\nint f(int x) { return x; }",
            vec!["purpose", "parameter:x", "return"],
        ),
        (
            "complete",
            "/** Compute square.\n * @param x input value\n * @return square\n */\nint f(int x) { return x*x; }",
            vec![],
        ),
        (
            "bare_param",
            "/** Compute result.\n * @param x\n * @return result\n */\nint f(int x) { return x; }",
            vec!["parameter:x"],
        ),
    ];
    let mut evidence = Vec::new();
    for (lang, std, ext) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        for (name, source, expected) in &cases {
            let file = root.join(format!("{name}.{ext}"));
            fs::write(&file, source).unwrap();
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["comments", lang])
                .arg(&file)
                .arg("--clang-tool")
                .arg(&clang)
                .args(["--standard", std, "--format=json"])
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(3));
            let report: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(report["documentation_structure"]["status"], "observed");
            assert_eq!(report["schema_version"], if report.get("placeholder_workbench").is_some() {"0.11.0"} else {"0.10.0"});
            assert_eq!(
                report["documentation_structure"]["observation"]["functions"][0]["missing_components"],
                json!(expected)
            );
            assert_eq!(report["coverage_proven"], false);
            assert_eq!(report["detailed_contract_qualification"], "not_granted");
            assert!(!root.join(".codeguard").exists());
            if *name == "bare_param" {
                assert_eq!(report["native"]["diagnostics"].as_array().unwrap().len(), 1);
                assert_eq!(
                    report["documentation_structure"]["native_raw_diagnostic_count"],
                    2
                );
            }
            evidence.push(json!({"language":lang,"case":name,"report":report}));
        }
    }
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&root)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    for (lang, standard, extension) in [("c", "c11", "c"), ("cpp", "c++17", "cpp")] {
        for missing_tool in [false, true] {
            let selected = if missing_tool {
                root.join("missing-clang")
            } else {
                clang.clone()
            };
            let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["comments", lang])
                .arg(root.join(format!("absent.{extension}")))
                .arg("--clang-tool")
                .arg(&selected)
                .args(["--standard", standard, "--format=json"])
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(3));
            let report: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(report["schema_version"], if report.get("placeholder_workbench").is_some() {"0.11.0"} else {"0.10.0"});
            assert_eq!(report["workspace_binding"], "bound");
            assert_eq!(report["structural_task_workflow_status"], "partial");
            if missing_tool {
                assert_eq!(report["documentation_structure"]["status"], "incomplete");
                assert_eq!(
                    report["documentation_structure"]["observation"],
                    Value::Null
                );
            } else {
                assert_eq!(report["documentation_structure"]["status"], "observed");
                assert_eq!(report["workbench"]["task_ids"], json!([]));
            }
            evidence.push(json!({"language":lang,"case":if missing_tool {"bound_missing_tool"}else{"bound_absent"},"report":report}));
        }
    }
    if let Some(path) = std::env::var_os("CODEGUARD_STRUCTURE_CLI_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"development_native_public_structure","qualification":"not_granted","test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_comments_structure_cli.rs"))),"codeguard_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"cases":evidence})).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
