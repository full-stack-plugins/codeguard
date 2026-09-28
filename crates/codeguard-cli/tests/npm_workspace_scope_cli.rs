#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn nested_npm_scopes_share_parent_workbench_without_merging_or_scanning_external_projects() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-scope-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    for name in ["a", "ab"] {
        fs::create_dir(fixture.0.join(name)).unwrap();
        fs::write(
            fixture.0.join(name).join("package.json"),
            r#"{"scripts":{"audit":"npm audit"}}"#,
        )
        .unwrap();
        fs::write(
            fixture.0.join(name).join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
        )
        .unwrap();
    }
    let node = fixture.0.join("node");
    let trace = fixture.0.join("trace");
    fs::write(&node,format!("#!/bin/sh\nprintf ran >> '{}'\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}'; fi\n",trace.display())).unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    for file in ["npm.js", "user.npmrc", "global.npmrc"] {
        fs::write(fixture.0.join(file), "").unwrap();
    }
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::write(
        fixture.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"8s"}"#,
    )
    .unwrap();
    fs::create_dir(fixture.0.join("a/.codeguard")).unwrap();
    fs::write(
        fixture.0.join("a/.codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"9s"}"#,
    )
    .unwrap();
    let paths: Vec<_> = ["npm.js", "user.npmrc", "global.npmrc"]
        .iter()
        .map(|s| fixture.0.join(s))
        .collect();
    let run = |project: &Path, workspace: &Path| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "cve",
                "typescript",
                project.to_str().unwrap(),
                "--workspace",
                workspace.to_str().unwrap(),
                "--node-tool",
                node.to_str().unwrap(),
                "--npm-entry",
                paths[0].to_str().unwrap(),
                "--npm-version",
                "11.16.0",
                "--userconfig",
                paths[1].to_str().unwrap(),
                "--globalconfig",
                paths[2].to_str().unwrap(),
                "--format",
                "json",
            ])
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
    for name in ["a", "ab", "a"] {
        let report = run(&fixture.0.join(name), &fixture.0);
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert_eq!(report["execution_budget"]["timeout_ms"], 8000);
        assert_eq!(report["execution_budget"]["source"], "project_default");
    }
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        2
    );
    assert!(!fixture.0.join("a/.codeguard/tasks").exists());
    assert!(!fixture.0.join("ab/.codeguard").exists());
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(next["repair_brief"]["checker_id"], "node.npm.audit");
    assert!(
        next["repair_brief"]["recheck_argv"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "--workspace")
    );
    let task_id = next["repair_brief"]["task_id"].as_str().unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task_id,
            fixture.0.to_str().unwrap(),
            "--node-tool",
            node.to_str().unwrap(),
            "--npm-entry",
            paths[0].to_str().unwrap(),
            "--npm-version",
            "11.16.0",
            "--userconfig",
            paths[1].to_str().unwrap(),
            "--globalconfig",
            paths[2].to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        verify.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let verification: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verification["event_persisted"], true, "{verification}");
    assert_eq!(verification["observation"], "still_blocked");
    assert_eq!(verification["native_scan"]["checker_id"], "node.npm.audit");
    assert_eq!(verification["native_scan"]["coverage_proven"], false);
    assert_eq!(verification["execution_budget"]["timeout_ms"], 8000);
    let refreshed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let refreshed: Value = serde_json::from_slice(&refreshed.stdout).unwrap();
    assert_eq!(refreshed["repair_brief"]["task_id"], task_id, "{refreshed}");
    assert_eq!(
        refreshed["repair_brief"]["verification_observation"], "still_blocked",
        "{refreshed}"
    );

    let task_command = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let claim = task_command(&[
        "task",
        "claim",
        task_id,
        fixture.0.to_str().unwrap(),
        "--owner",
        "npm-agent",
        "--format",
        "json",
    ]);
    let token = claim["lease_token"].as_str().unwrap();
    let started = task_command(&[
        "task",
        "attempt",
        "start",
        task_id,
        fixture.0.to_str().unwrap(),
        "--owner",
        "npm-agent",
        "--lease-token",
        token,
        "--action-id",
        refreshed["repair_brief"]["action_id"].as_str().unwrap(),
        "--format",
        "json",
    ]);
    task_command(&[
        "task",
        "attempt",
        "finish",
        task_id,
        fixture.0.to_str().unwrap(),
        "--owner",
        "npm-agent",
        "--lease-token",
        token,
        "--attempt-id",
        started["attempt_id"].as_str().unwrap(),
        "--outcome",
        "ready-to-verify",
        "--note-code",
        "no_change",
        "--format",
        "json",
    ]);
    fs::write(&node, "#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{\"error\":{\"code\":\"EAUDITNOLOCK\"}}'; exit 1; fi\n").unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task_id,
            fixture.0.to_str().unwrap(),
            "--owner",
            "npm-agent",
            "--lease-token",
            token,
            "--node-tool",
            node.to_str().unwrap(),
            "--npm-entry",
            paths[0].to_str().unwrap(),
            "--npm-version",
            "11.16.0",
            "--userconfig",
            paths[1].to_str().unwrap(),
            "--globalconfig",
            paths[2].to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(3));
    let failed: Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(failed["event_persisted"], true, "{failed}");
    assert_eq!(failed["observation"], "incomplete");
    let after_failure = task_command(&["next", fixture.0.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        after_failure["repair_brief"]["verification_observation"], "incomplete",
        "{after_failure}"
    );
    assert_eq!(
        after_failure["repair_brief"]["history"]["awaiting_verification"], false,
        "{after_failure}"
    );
    let before = fs::read(&trace).unwrap();
    let invalid = run(&fixture.0.join("ab"), &fixture.0.join("a"));
    assert_eq!(invalid["reason"], "npm_scope_outside_workspace");
    assert_eq!(fs::read(&trace).unwrap(), before);
    symlink(fixture.0.join("a"), fixture.0.join("linked-workspace")).unwrap();
    let invalid = run(&fixture.0.join("a"), &fixture.0.join("linked-workspace"));
    assert_eq!(invalid["reason"], "npm_workspace_invalid");
    assert_eq!(fs::read(&trace).unwrap(), before);

    // 即使本地事件被一并改写摘要，损坏/重复字段也不能被当成输入过期静默忽略。
    let run_id = failed["native_scan"]["run_id"].as_str().unwrap();
    let report_path = fixture.0.join(format!(".codeguard/reports/{run_id}.json"));
    let event_path = fixture.0.join(format!(
        ".codeguard/findings/{task_id}/events/verify-{run_id}.json"
    ));
    let original_report = fs::read(&report_path).unwrap();
    let original_event = fs::read(&event_path).unwrap();
    let marker_path = fixture
        .0
        .join(format!(".codeguard/state/consumed/{run_id}.json"));
    let original_marker = fs::read(&marker_path).unwrap();
    let mut malformed: Value = serde_json::from_slice(&original_report).unwrap();
    malformed["coverage_proven"] = serde_json::json!(true);
    let mut wrong_scope: Value = serde_json::from_slice(&original_report).unwrap();
    wrong_scope["build_root"] = serde_json::json!(if wrong_scope["build_root"] == "a" {
        "ab"
    } else {
        "a"
    });
    let mut wrong_workspace: Value = serde_json::from_slice(&original_report).unwrap();
    wrong_workspace["workspace_id"] = serde_json::json!("ws-11111111111111111111111111111111");
    let mut wrong_count: Value = serde_json::from_slice(&original_report).unwrap();
    wrong_count["component_count"] = serde_json::json!(1);
    let duplicate = format!(
        "{{\"authority\":\"approved\",{}",
        std::str::from_utf8(&original_report)
            .unwrap()
            .trim_start_matches('{')
    )
    .into_bytes();
    let input_path = fixture
        .0
        .join(next["repair_brief"]["scope"].as_str().unwrap())
        .join("package.json");
    let original_manifest = fs::read(&input_path).unwrap();
    for changed_input in [false, true] {
        if changed_input {
            fs::write(&input_path, "{}").unwrap();
        }
        for bytes in [
            serde_json::to_vec(&wrong_scope).unwrap(),
            serde_json::to_vec(&wrong_workspace).unwrap(),
            serde_json::to_vec(&malformed).unwrap(),
            serde_json::to_vec(&wrong_count).unwrap(),
            duplicate.clone(),
        ] {
            use sha2::{Digest, Sha256};
            fs::write(&report_path, &bytes).unwrap();
            let mut event: Value = serde_json::from_slice(&original_event).unwrap();
            event["report_sha256"] = serde_json::json!(format!("{:x}", Sha256::digest(&bytes)));
            fs::write(&event_path, serde_json::to_vec(&event).unwrap()).unwrap();
            let mut marker: Value = serde_json::from_slice(&original_marker).unwrap();
            marker["report_sha256"] = event["report_sha256"].clone();
            fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
            let invalid = Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
                .output()
                .unwrap();
            let invalid: Value = serde_json::from_slice(&invalid.stdout).unwrap();
            assert_eq!(invalid["reason"], "verification_event_invalid", "{invalid}");
            fs::write(&report_path, &original_report).unwrap();
            fs::write(&event_path, &original_event).unwrap();
            fs::write(&marker_path, &original_marker).unwrap();
        }
        fs::write(&input_path, &original_manifest).unwrap();
    }
    let scope = next["repair_brief"]["scope"].as_str().unwrap();
    fs::write(fixture.0.join(scope).join("package.json"), "{}").unwrap();
    let stale = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let stale: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale["repair_brief"]["task_id"], task_id, "{stale}");
    assert!(
        stale["repair_brief"]["verification_observation"].is_null(),
        "{stale}"
    );
}
