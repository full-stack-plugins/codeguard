#![cfg(unix)]
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn deleted_manifest_is_kept_as_preparation_scope_without_reusing_old_input_or_authority() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-historical-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let root = &fixture.0;
    for name in ["a", "ab"] {
        fs::create_dir(root.join(name)).unwrap();
        fs::write(
            root.join(name).join("package.json"),
            r#"{"scripts":{"audit":"npm audit"}}"#,
        )
        .unwrap();
        fs::write(
            root.join(name).join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
        )
        .unwrap();
    }
    let run = |args: &[&str]| {
        let p = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(p.status.code(), Some(3));
        serde_json::from_slice::<Value>(&p.stdout).unwrap()
    };
    let path = root.to_str().unwrap();
    run(&["init", path, "--apply", "--format", "json"]);
    fs::remove_file(root.join("a/package.json")).unwrap();
    for count in [1, 0] {
        let report = run(&["check", "all", path, "--format", "json"]);
        let scans = report["native_results"]["npm_cve"].as_array().unwrap();
        assert_eq!(scans.len(), 2, "{report}");
        let missing = scans.iter().find(|s| s["build_root"] == "a").unwrap();
        assert_eq!(missing["feedback"]["reason"], "npm_manifest_missing");
        assert_eq!(missing["observation"]["manifest_state"], "missing");
        assert!(missing["observation"]["manifest_sha256"].is_null());
        assert_eq!(missing["backlog_status"], "synced_partial");
        assert!(
            missing["feedback"]["next_action"]
                .as_str()
                .unwrap()
                .starts_with("历史本地记录仅定位")
        );
        assert_eq!(missing["feedback"]["workbench"]["new_blockers"], count);
        assert_eq!(
            fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
            2
        );
    }
    // 同一请求有完整参数时，只允许当前发现的根运行工具，历史根仍是准备。
    use std::os::unix::fs::PermissionsExt;
    let node = root.join("node");
    let trace = root.join("trace");
    fs::write(&node,format!("#!/bin/sh\nprintf '%s\\n' \"$PWD\" >> '{}'\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}'; fi\n",trace.display())).unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    let entry = root.join("npm.js");
    let user = root.join("user.npmrc");
    let global = root.join("global.npmrc");
    for file in [&entry, &user, &global] {
        fs::write(file, "").unwrap();
    }
    let mixed = run(&[
        "check",
        "all",
        path,
        "--node-tool",
        node.to_str().unwrap(),
        "--npm-entry",
        entry.to_str().unwrap(),
        "--npm-version",
        "11.16.0",
        "--userconfig",
        user.to_str().unwrap(),
        "--globalconfig",
        global.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let scans = mixed["native_results"]["npm_cve"].as_array().unwrap();
    assert_eq!(
        scans.iter().find(|s| s["build_root"] == "a").unwrap()["feedback"]["reason"],
        "npm_manifest_missing"
    );
    assert_eq!(
        scans.iter().find(|s| s["build_root"] == "ab").unwrap()["feedback"]["local_coherent"],
        true,
        "{mixed}"
    );
    let calls = fs::read_to_string(trace).unwrap();
    assert_eq!(calls.lines().count(), 2);
    assert!(
        calls
            .lines()
            .all(|cwd| cwd == root.join("ab").to_str().unwrap())
    );
    let profile = root.join("codeguard/project.json");
    let bytes = fs::read(&profile).unwrap();
    let mut bad: Value = serde_json::from_slice(&bytes).unwrap();
    bad["manifest_sha256"]["../outside/package.json"] = serde_json::json!("0".repeat(64));
    fs::write(&profile, bad.to_string()).unwrap();
    let rejected = run(&["check", "all", path, "--format", "json"]);
    assert_eq!(
        rejected["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        rejected["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "historical_npm_scope_unavailable")
    );
    // 项目自行同时改画像与本地摘要，也不能扩展到越界路径或改变协议身份。
    use sha2::{Digest, Sha256};
    let workspace = root.join("codeguard/workspace.json");
    let workspace_bytes = fs::read(&workspace).unwrap();
    let invalid_profile = fs::read(&profile).unwrap();
    let mut rewritten: Value = serde_json::from_slice(&workspace_bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(&invalid_profile));
    rewritten["project_sha256"] = serde_json::json!(hash);
    rewritten["managed_sha256"]["project.json"] = rewritten["project_sha256"].clone();
    fs::write(&workspace, rewritten.to_string()).unwrap();
    let still_rejected = run(&["check", "all", path, "--format", "json"]);
    assert_eq!(
        still_rejected["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        still_rejected["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "historical_npm_scope_unavailable")
    );
    for mode in ["alias", "wrong_type", "duplicate"] {
        let mut candidate: Value = serde_json::from_slice(&bytes).unwrap();
        let candidate_bytes = match mode {
            "alias" => {
                candidate["manifest_sha256"]["a//package.json"] = serde_json::json!("0".repeat(64));
                candidate.to_string().into_bytes()
            }
            "wrong_type" => {
                candidate["document_type"] = serde_json::json!("other_profile");
                candidate.to_string().into_bytes()
            }
            _ => candidate
                .to_string()
                .replacen("{", "{\"manifest_sha256\":{},", 1)
                .into_bytes(),
        };
        fs::write(&profile, &candidate_bytes).unwrap();
        let mut binding: Value = serde_json::from_slice(&workspace_bytes).unwrap();
        binding["project_sha256"] =
            serde_json::json!(format!("{:x}", Sha256::digest(&candidate_bytes)));
        binding["managed_sha256"]["project.json"] = binding["project_sha256"].clone();
        fs::write(&workspace, binding.to_string()).unwrap();
        let rejected = run(&["check", "all", path, "--format", "json"]);
        assert_eq!(
            rejected["native_results"]["npm_cve"]
                .as_array()
                .unwrap()
                .len(),
            2,
            "mode={mode}, {rejected}"
        );
        assert!(
            rejected["unresolved_conditions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "historical_npm_scope_unavailable")
        );
        assert_eq!(
            fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
            2
        );
    }
    fs::write(&workspace, workspace_bytes).unwrap();
    fs::write(&profile, bytes).unwrap();
    fs::write(
        root.join("a/package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    let recovered = run(&["check", "all", path, "--format", "json"]);
    assert_eq!(
        recovered["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        fs::read_dir(root.join("codeguard/tasks")).unwrap().count(),
        2
    );
}
