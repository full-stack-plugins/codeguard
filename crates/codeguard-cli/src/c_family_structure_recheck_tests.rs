//! 原工具结构复检真实测试；本地观察不证明生产资格。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires installed Clang and explicitly built Codeguard; no tool download"]
fn original_structure_recheck_preserves_task_and_rejects_changed_origin() {
    let cli = std::env::var_os("CODEGUARD_RECHECK_CLI").expect("explicit built CLI");
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("existing Clang"));
    let root = std::env::temp_dir().join(format!(
        "cg-struct-recheck-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let root = root.canonicalize().unwrap();
    let source = root.join("sample.c");
    fs::write(&source, "int f(int x) { return x; }\n").unwrap();
    assert_eq!(
        Command::new(&cli)
            .args(["init"])
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    let scan = Command::new(&cli)
        .args(["comments", "c"])
        .arg(&source)
        .arg("--clang-tool")
        .arg(&tool)
        .args(["--standard", "c11"])
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(
        scan.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );
    let report: Value = serde_json::from_slice(&scan.stdout).unwrap();
    let brief = &report["structural_next"]["repair_brief"];
    assert!(brief["task_id"].is_string());
    let deadline = || Instant::now() + Duration::from_secs(30);
    let first = crate::c_family_structure_task_recheck::run(&root, brief, deadline()).unwrap();
    assert_eq!(
        crate::c_family_structure_task_recheck::classify(brief, &first),
        "still_present"
    );
    assert!(crate::c_family_structure_task_recheck::inputs_current(
        &root, &first
    ));
    fs::write(&source,"/** Return input.\n * @param x input value.\n * @return unchanged input.\n */\nint f(int x) { return x; }\n").unwrap();
    assert!(!crate::c_family_structure_task_recheck::inputs_current(
        &root, &first
    ));
    let clean = crate::c_family_structure_task_recheck::run(&root, brief, deadline()).unwrap();
    assert_eq!(
        crate::c_family_structure_task_recheck::classify(brief, &clean),
        "candidate_absent_unverified_policy"
    );
    let expired =
        crate::c_family_structure_task_recheck::run(&root, brief, Instant::now()).unwrap();
    assert_eq!(
        crate::c_family_structure_task_recheck::classify(brief, &expired),
        "incomplete"
    );
    fs::write(&source, "int f(int x) { return ;\n").unwrap();
    let syntax_error =
        crate::c_family_structure_task_recheck::run(&root, brief, deadline()).unwrap();
    assert_eq!(
        crate::c_family_structure_task_recheck::classify(brief, &syntax_error),
        "incomplete"
    );
    let mut forged = clean.clone();
    forged["task_rule"] = "clang.fake".into();
    assert!(!crate::c_family_structure_task_recheck::valid_shape(
        &root, &forged
    ));
    assert!(
        crate::c_family_structure_task_recheck::preflight(
            &root,
            brief,
            Some(&root.join("other-clang"))
        )
        .is_err()
    );
    let origin = root.join(format!(
        ".codeguard/reports/{}.json",
        brief["evidence_ref"]["first_run_id"].as_str().unwrap()
    ));
    let mut bytes = fs::read(&origin).unwrap();
    bytes.push(b'\n');
    fs::write(&origin, bytes).unwrap();
    assert!(crate::c_family_structure_task_recheck::run(&root, brief, deadline()).is_err());
    if let Some(path) = std::env::var_os("CODEGUARD_STRUCTURE_RECHECK_EVIDENCE") {
        let evidence = json!({"evidence_kind":"development_native_structural_recheck","qualification":"not_granted","helper_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_structure_task_recheck.rs"))),"test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_family_structure_recheck_tests.rs"))),"first":first,"clean":clean,"expired":expired,"syntax_error":syntax_error,"negative_cases":["source_changed_after_observation","forged_structural_rule","different_selected_tool","altered_first_report"]});
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
