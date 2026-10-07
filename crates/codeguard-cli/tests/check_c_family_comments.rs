//! 统一检查的真实C/C++文档链路；显式档案不证明完整项目覆盖。
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
#[ignore = "requires explicit installed Clang21"]
fn check_c_family_preserves_original_documentation_and_stable_tasks() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("installed Clang"));
    let root = std::env::temp_dir().join(format!(
        "cg-check-c-doc-{}-{}",
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
    fs::write(root.join("a.c"), "int f(int x) { return x; }\n").unwrap();
    fs::write(root.join("b.cpp"), "int g(int x) { return x; }\n").unwrap();
    let run = |language: &str, options: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", language])
            .arg(&root)
            .args(options)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice::<Value>(&o.stdout).unwrap()
    };
    let options = [
        "--clang-tool",
        tool.to_str().unwrap(),
        "--c-standard",
        "c11",
        "--cpp-standard",
        "c++17",
        "--jobs",
        "4",
    ];
    let unbound = run("all", &options);
    assert_eq!(
        unbound["native_results"]["c_family_comments"]["c"]["files"][0]["feedback"]["workspace_binding"],
        "not_bound"
    );
    assert!(!root.join(".codeguard").exists());
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    let first = run("all", &options);
    let repeated = run("all", &options);
    for lang in ["c", "cpp"] {
        let scan = &first["native_results"]["c_family_comments"][lang];
        assert_eq!(scan["local_scan_complete"], true);
        assert_eq!(scan["project_configuration"], "unknown");
        let feedback = &scan["files"][0]["feedback"];
        let ids = &feedback["structural_workbench"]["task_ids"];
        assert_eq!(ids.as_array().unwrap().len(), 1);
        assert_eq!(
            ids,
            &repeated["native_results"]["c_family_comments"][lang]["files"][0]["feedback"]["structural_workbench"]
                ["task_ids"]
        );
        assert_eq!(
            feedback["structural_next"]["repair_brief"]["rule_source"],
            "codeguard_structural_policy"
        );
    }
    let c = run(
        "c",
        &[
            "--clang-tool",
            tool.to_str().unwrap(),
            "--c-standard",
            "c11",
        ],
    );
    assert!(
        c["native_results"]["c_family_comments"]
            .get("cpp")
            .is_none()
    );
    assert_eq!(
        c["next"]["repair_brief"]["checker_id"],
        "c.clang.documentation_structure"
    );
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "c"])
        .arg(&root)
        .arg("--clang-tool")
        .arg(&tool)
        .args(["--c-standard", "c11", "--format=human"])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("结构缺口 1 项"));
    assert!(human.contains("完整项目配置未确认"));
    let exported = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "c"])
        .arg(&root)
        .arg("--clang-tool")
        .arg(&tool)
        .args(["--c-standard", "c11", "--format=sarif"])
        .output()
        .unwrap();
    assert_eq!(exported.status.code(), Some(3));
    let sarif: Value = serde_json::from_slice(&exported.stdout).unwrap();
    assert_eq!(
        sarif["runs"][0]["properties"]["structuralPolicyFindingCount"],
        1
    );
    assert_eq!(sarif["runs"][0]["properties"]["nativeFindingCount"], 0);
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["codeguardObservation"],
        "structural_policy_unverified"
    );
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    let missing_tool = run(
        "c",
        &[
            "--clang-tool",
            root.join("absent-clang").to_str().unwrap(),
            "--c-standard",
            "c11",
        ],
    );
    assert_eq!(
        missing_tool["native_results"]["c_family_comments"]["c"]["local_scan_complete"],
        false
    );
    assert!(missing_tool["native_results"]["c_family_comments"]["c"]["files"][0]["feedback"]["documentation_findings"].as_array().unwrap().is_empty());
    let deadline = run(
        "c",
        &[
            "--clang-tool",
            tool.to_str().unwrap(),
            "--c-standard",
            "c11",
            "--timeout",
            "1ms",
        ],
    );
    assert_eq!(
        deadline["native_results"]["c_family_comments"]["c"]["local_scan_complete"],
        false
    );
    let missing_context = run("c", &[]);
    assert_eq!(
        missing_context["native_results"]["c_family_comments"]["c"]["status"],
        "context_required"
    );
    assert!(
        first["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == "c.comments")
    );
    assert_eq!(first["delivery_decision"], "incomplete");
    if let Some(path) = std::env::var_os("CODEGUARD_CHECK_C_DOCUMENTATION_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"evidence_kind":"development_check_c_family_documentation","qualification":"not_granted","test_source_sha256":format!("{:x}",Sha256::digest(fs::read(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/check_c_family_comments.rs")).unwrap())),"binary_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),"human":human,"sarif":sarif,"missing_tool":missing_tool,"deadline":deadline,"first":first,"repeated":repeated,"selected":c,"unbound":unbound,"missing_context":missing_context})).unwrap()).unwrap();
    }
}

#[test]
fn invalid_c_documentation_options_fail_before_process_execution() {
    for args in [
        vec![
            "check",
            "c",
            "--clang-tool",
            "relative",
            "--c-standard",
            "c11",
        ],
        vec![
            "check",
            "c",
            "--clang-tool",
            "/absent",
            "--c-standard",
            "c17",
        ],
        vec!["check", "c", "--c-standard", "c11"],
        vec![
            "check",
            "java",
            "--clang-tool",
            "/absent",
            "--c-standard",
            "c11",
        ],
        vec![
            "check",
            "cpp",
            "--clang-tool",
            "/absent",
            "--c-standard",
            "c11",
        ],
        vec![
            "check",
            "all",
            "--clang-tool",
            "/absent",
            "--clang-tool",
            "/absent",
            "--c-standard",
            "c11",
        ],
        vec![
            "lint",
            "all",
            "--clang-tool",
            "/absent",
            "--c-standard",
            "c11",
        ],
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(o.stdout.is_empty());
    }
}

#[cfg(unix)]
#[test]
#[ignore = "requires explicit installed Clang21"]
fn project_input_change_retracts_c_documentation_before_task_persistence() {
    use std::os::unix::fs::PermissionsExt;
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("installed Clang"));
    let root = std::env::temp_dir().join(format!(
        "cg-check-c-doc-mutation-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(root.join("a.c"), "int f(int x) { return x; }\n").unwrap();
    fs::write(root.join("b.cpp"), "int g(int x) { return x; }\n").unwrap();
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&root)
            .args(["--apply", "--format=json"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    let wrapper = root.join("clang-wrapper");
    let quote = |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\"'\"'"));
    fs::write(&wrapper,format!("#!/bin/sh\nif [ \"$1\" = '--version' ]; then exec {} \"$@\"; fi\n{} \"$@\"\nresult=$?\nprintf 'int changed(int x) {{ return x + 1; }}\\n' > {}\nexit \"$result\"\n",quote(&tool),quote(&tool),quote(&root.join("b.cpp")))).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(&root)
        .arg("--clang-tool")
        .arg(&wrapper)
        .args([
            "--c-standard",
            "c11",
            "--cpp-standard",
            "c++17",
            "--jobs",
            "4",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    for lang in ["c", "cpp"] {
        let scan = &r["native_results"]["c_family_comments"][lang];
        assert_eq!(scan["scope_stable"], false);
        assert_eq!(scan["local_scan_complete"], false);
        for row in scan["files"].as_array().unwrap() {
            assert_eq!(row["current"], false);
            assert_eq!(
                row["feedback"]["documentation_findings"],
                serde_json::json!([])
            );
            assert_eq!(
                row["feedback"]["documentation_structure"]["observation"],
                Value::Null
            );
            assert_eq!(row["feedback"]["next"], Value::Null);
        }
    }
    assert_eq!(
        fs::read_dir(root.join(".codeguard/findings"))
            .unwrap()
            .count(),
        0
    );
    if let Some(path) = std::env::var_os("CODEGUARD_CHECK_C_DOCUMENTATION_MUTATION_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn check_c_documentation_sigint_uses_shared_cancellation_and_preserves_incomplete_feedback() {
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
        process::Stdio,
    };
    let root = std::env::temp_dir().join(format!(
        "cg-check-c-doc-cancel-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(root.join("a.c"), "int f(int x) { return x; }\n").unwrap();
    let marker = root.join("scanning");
    let ghost = root.join("ghost");
    let tool = root.join("clang");
    let quote = |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\"'\"'"));
    fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'Apple clang version 21.0.0 (clang-2100.3.34.2)'; exit 0; fi\ncat >/dev/null\n: > {}\n(sleep 2; printf ghost > {}) &\nsleep 10\n",quote(&marker),quote(&ghost))).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "c"])
        .arg(&root)
        .arg("--clang-tool")
        .arg(&tool)
        .args(["--c-standard", "c11", "--timeout", "20s", "--format=json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while !marker.exists() {
        assert!(child.try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let o = child.wait_with_output().unwrap();
    assert_eq!(
        o.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r["command_status"], "cancelled");
    assert_eq!(r["next"], Value::Null);
    let scan = &r["native_results"]["c_family_comments"]["c"];
    assert_eq!(scan["local_scan_complete"], false);
    assert_eq!(scan["scope_stable"], false);
    assert_eq!(
        scan["files"][0]["feedback"]["native"]["reason"],
        "request_cancelled"
    );
    assert!(!root.join(".codeguard").exists());
    std::thread::sleep(Duration::from_millis(2200));
    assert!(!ghost.exists());
    if let Some(path) = std::env::var_os("CODEGUARD_CHECK_C_DOCUMENTATION_CANCEL_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    }
}

#[test]
fn check_c_documentation_file_limit_retains_unobserved_tail_without_initializing_workspace() {
    let root = std::env::temp_dir().join(format!(
        "cg-check-c-doc-limit-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    for i in 0..65 {
        fs::write(
            root.join(format!("api_{i:03}.c")),
            "int f(int x) { return x; }\n",
        )
        .unwrap();
    }
    let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "c"])
        .arg(&root)
        .arg("--clang-tool")
        .arg(root.join("missing-clang"))
        .args(["--c-standard", "c11", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let r: Value = serde_json::from_slice(&o.stdout).unwrap();
    let scan = &r["native_results"]["c_family_comments"]["c"];
    assert_eq!(scan["source_file_count"], 65);
    assert_eq!(scan["files"].as_array().unwrap().len(), 64);
    assert_eq!(scan["unobserved_count"], 1);
    assert_eq!(scan["local_scan_complete"], false);
    assert_eq!(scan["native_task_started"], false);
    assert!(!root.join(".codeguard").exists());
    if let Some(path) = std::env::var_os("CODEGUARD_CHECK_C_DOCUMENTATION_LIMIT_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    }
}
