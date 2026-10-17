use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN"]
fn cpp_methods_and_constructor_keep_stable_tasks_through_original_rechecks() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("existing Clang"));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-cpp-member-work-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _fixture = Fixture(root.clone());
    let source = root.join("api.cpp");
    let initial = "struct Box {\nint read(int x);\n/// Create the box.\nBox(int size);\n};\n";
    fs::write(&source, initial).unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .arg("init")
        .arg(&root)
        .args(["--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let scan = || {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["comments", "cpp"])
            .arg(&source)
            .arg("--clang-tool")
            .arg(&tool)
            .args(["--standard", "c++17", "--format=json"])
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
    let first = scan();
    let ids = first["structural_workbench"]["task_ids"]
        .as_array()
        .unwrap();
    assert_eq!(ids.len(), 1, "{first}");
    let id = ids[0].as_str().unwrap();
    assert_eq!(
        first["structural_next"]["repair_brief"]["structural_positions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        first["structural_next"]["repair_brief"]["allowed_paths"],
        json!(["api.cpp"])
    );
    assert_eq!(scan()["structural_workbench"]["task_ids"], json!([id]));
    let verify = || {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["task", "verify"])
            .arg(id)
            .arg(&root)
            .arg("--clang-tool")
            .arg(&tool)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["event_persisted"], true);
        assert_eq!(report["task_id"], id);
        assert_eq!(report["native_scan"]["standard"], "c++17");
        assert_eq!(report["native_scan"]["selected_tool"], json!(tool));
        assert_eq!(report["native_scan"]["input_stable"], true);
        assert_eq!(report["native_scan"]["coverage_proven"], false);
        assert_eq!(
            report["native_scan"]["source_sha256"],
            format!("{:x}", Sha256::digest(fs::read(&source).unwrap()))
        );
        report
    };
    let present = verify();
    assert_eq!(present["observation"], "still_present");
    let fixed = "struct Box {\n/// Read a value.\n/// @param x Input value.\n/// @return The value.\nint read(int x);\n/// Create the box.\n/// @param size Initial size.\nBox(int size);\n};\n";
    fs::write(&source, fixed).unwrap();
    let repaired = verify();
    assert_eq!(
        repaired["observation"],
        "candidate_absent_unverified_policy"
    );
    let fact_path = root.join(format!(".codeguard/findings/{id}/finding.json"));
    let fact: Value = serde_json::from_slice(&fs::read(&fact_path).unwrap()).unwrap();
    assert_eq!(fact["state"], "open");
    fs::write(&source, initial).unwrap();
    let recurrence = verify();
    assert_eq!(recurrence["observation"], "still_present");
    assert_eq!(scan()["structural_workbench"]["task_ids"], json!([id]));
    if let Ok(destination) = std::env::var("CODEGUARD_CPP_WORKBENCH_EVIDENCE") {
        let path = std::path::Path::new(&destination);
        assert!(path.is_absolute());
        let evidence = json!({
            "qualification":"not_granted", "scope":"cpp17_member_structural_tasks_not_trusted_closure",
            "test_source_sha256":format!("{:x}", Sha256::digest(include_bytes!("cpp_documentation_workbench.rs"))),
            "cli_sha256":format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),
            "initial_source_sha256":format!("{:x}", Sha256::digest(initial.as_bytes())),
            "fixed_source_sha256":format!("{:x}", Sha256::digest(fixed.as_bytes())),
            "stable_task_id":id, "first":first, "present":present, "repaired":repaired, "recurrence":recurrence
        });
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
