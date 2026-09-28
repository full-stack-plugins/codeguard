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
fn public_cve_command_shows_preparation_native_counts_and_unverified_coverage() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-cli-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(
        fixture.0.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"}}"#,
    )
    .unwrap();
    fs::write(
        fixture.0.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let execute = |extra: &[&str]| {
        let mut args = vec![
            "cve",
            "typescript",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ];
        args.extend_from_slice(extra);
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap()
    };
    let missing = execute(&[]);
    assert_eq!(missing.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(report["reason"], "npm_execution_context_missing");
    let node = fixture.0.join("node");
    fs::write(&node,"#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{\"auditReportVersion\":2,\"vulnerabilities\":{},\"metadata\":{\"vulnerabilities\":{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0},\"dependencies\":{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}'; fi\n").unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    for file in ["npm.js", "user.npmrc", "global.npmrc"] {
        fs::write(fixture.0.join(file), "").unwrap();
    }
    let paths: Vec<_> = ["node", "npm.js", "user.npmrc", "global.npmrc"]
        .iter()
        .map(|f| fixture.0.join(f))
        .collect();
    let result = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    assert_eq!(result.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["local_coherent"], true);
    assert_eq!(report["advisory_coverage"], "not_evaluated");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["dependency_total"], 0);
    assert_eq!(report["configuration"]["configuration"], "configured");
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(
        execute(&["--npm-version", "11.16.0", "--npm-version", "11.16.0"])
            .status
            .code(),
        Some(2)
    );
    assert!(!fixture.0.join("codeguard").exists());
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    // 缺原生上下文也必须留下稳定环境任务；不能伪造执行或漏洞。
    for count in [1, 0] {
        let missing = execute(&[]);
        let report: Value = serde_json::from_slice(&missing.stdout).unwrap();
        assert_eq!(report["reason"], "npm_execution_context_missing");
        assert_eq!(report["local_coherent"], false);
        assert_eq!(report["findings"], serde_json::json!([]));
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert_eq!(report["workbench"]["new_blockers"], count);
    }
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    let task_id = next["repair_brief"]["task_id"].as_str().unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task_id,
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let verification: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verify.status.code(), Some(3));
    assert_eq!(verification["event_persisted"], true, "{verification}");
    assert_eq!(verification["observation"], "incomplete");
    assert_eq!(
        verification["native_scan"]["diagnostic_reason"],
        "npm_execution_context_missing"
    );
    assert_eq!(verification["native_scan"]["component_count"], 0);
    for count in [0, 0] {
        let result = execute(&[
            "--node-tool",
            paths[0].to_str().unwrap(),
            "--npm-entry",
            paths[1].to_str().unwrap(),
            "--userconfig",
            paths[2].to_str().unwrap(),
            "--globalconfig",
            paths[3].to_str().unwrap(),
            "--npm-version",
            "11.16.0",
        ]);
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert_eq!(report["workbench"]["new_blockers"], count);
    }
    let tasks: Vec<_> = fs::read_dir(fixture.0.join("codeguard/tasks"))
        .unwrap()
        .collect();
    assert_eq!(tasks.len(), 1);
    let task = fs::read_to_string(tasks[0].as_ref().unwrap().path()).unwrap();
    assert!(task.contains("关闭条件"));
    assert!(task.contains("局部零发现不关闭"));
    let next = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["next", fixture.0.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        next["repair_brief"]["checker_id"], "node.npm.audit",
        "{next}"
    );

    let advisory = serde_json::json!({"auditReportVersion":2,"vulnerabilities":{"fixture-pkg":{"name":"fixture-pkg","severity":"high","isDirect":true,"via":[{"source":123,"name":"fixture-pkg","dependency":"fixture-pkg","title":"fixture","url":"https://example.invalid","severity":"high","range":"<2.0.0"}],"effects":[],"range":"<2.0.0","nodes":["node_modules/fixture-pkg"],"fixAvailable":false}},"metadata":{"vulnerabilities":{"info":0,"low":0,"moderate":0,"high":1,"critical":0,"total":1},"dependencies":{"prod":2,"dev":0,"optional":0,"peer":0,"peerOptional":0,"total":1}}});
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; exit 1; fi\n",advisory)).unwrap();
    fs::write(
        fixture.0.join("package.json"),
        r#"{"scripts":{"audit":"npm audit"},"dependencies":{"fixture-pkg":"1.2.3"}}"#,
    )
    .unwrap();
    fs::write(fixture.0.join("package-lock.json"),r#"{"lockfileVersion":3,"packages":{"":{"dependencies":{"fixture-pkg":"1.2.3"}},"node_modules/fixture-pkg":{"version":"1.2.3"}}}"#).unwrap();
    let positive = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let positive: Value = serde_json::from_slice(&positive.stdout).unwrap();
    assert_eq!(positive["workbench_status"], "synced_partial", "{positive}");
    assert_eq!(
        positive["findings"].as_array().unwrap().len(),
        1,
        "{positive}"
    );
    assert_eq!(positive["workbench"]["new_blockers"], 0);
    assert_eq!(
        positive["findings"][0]["nodes"][0]["resolved_version"],
        "1.2.3"
    );
    // 原生命令异常退出时，完整且与锁文件绑定的漏洞报告仍需反馈给智能体。
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; exit 2; fi\n",advisory)).unwrap();
    let partial = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let partial: Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert_eq!(
        partial["reason"], "npm_audit_execution_incomplete",
        "{partial}"
    );
    assert_eq!(
        partial["findings"].as_array().unwrap().len(),
        1,
        "{partial}"
    );
    assert_eq!(partial["local_coherent"], true);
    assert_eq!(partial["delivery_decision"], "not_evaluated");
    assert_eq!(partial["workbench_status"], "synced_partial");
    assert_eq!(partial["workbench"]["new_blockers"], 0);
    fs::write(&node,"#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{\"auditReportVersion\":2'; exit 2; fi\n").unwrap();
    let malformed = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let malformed: Value = serde_json::from_slice(&malformed.stdout).unwrap();
    assert_eq!(malformed["findings"], serde_json::json!([]));
    assert_eq!(malformed["local_coherent"], false);
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; /bin/sleep 5; fi\n",advisory)).unwrap();
    let timed = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
        "--timeout",
        "2s",
    ]);
    let timed: Value = serde_json::from_slice(&timed.stdout).unwrap();
    assert_eq!(timed["reason"], "deadline", "{timed}");
    assert_eq!(timed["findings"].as_array().unwrap().len(), 1, "{timed}");
    assert_eq!(timed["delivery_decision"], "not_evaluated");
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; printf x >> \"$0\"; exit 2; fi\n",advisory)).unwrap();
    let changed = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let changed: Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(changed["reason"], "npm_input_changed", "{changed}");
    assert_eq!(changed["findings"], serde_json::json!([]));
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; /bin/sleep 0.2; /usr/bin/head -c 9000000 /dev/zero >&2; /bin/sleep 5; fi\n",advisory)).unwrap();
    let limited = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let limited: Value = serde_json::from_slice(&limited.stdout).unwrap();
    assert_eq!(limited["reason"], "npm_audit_output_limit", "{limited}");
    assert_eq!(
        limited["findings"].as_array().unwrap().len(),
        1,
        "{limited}"
    );
    assert_eq!(limited["delivery_decision"], "not_evaluated");
    fs::write(&node,"#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{\"auditReportVersion\":2'; /bin/sleep 0.2; /usr/bin/head -c 9000000 /dev/zero >&2; /bin/sleep 5; fi\n").unwrap();
    let truncated = execute(&[
        "--node-tool",
        paths[0].to_str().unwrap(),
        "--npm-entry",
        paths[1].to_str().unwrap(),
        "--userconfig",
        paths[2].to_str().unwrap(),
        "--globalconfig",
        paths[3].to_str().unwrap(),
        "--npm-version",
        "11.16.0",
    ]);
    let truncated: Value = serde_json::from_slice(&truncated.stdout).unwrap();
    assert_eq!(truncated["reason"], "npm_audit_output_limit", "{truncated}");
    assert_eq!(truncated["findings"], serde_json::json!([]));
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s' '{}'; exit 1; fi\n",advisory)).unwrap();
    let reports = fixture.0.join("codeguard/reports");
    let original = fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap();
    let mut malformed: Value = serde_json::from_slice(&fs::read(&original).unwrap()).unwrap();
    malformed["run_id"] = serde_json::json!("npm-bad-node");
    malformed["component_count"] = serde_json::json!(1);
    malformed["advisories"] = serde_json::json!([{"component_ref":"0".repeat(64),"severity":"high","advisory_sources":[123],"nodes":[{"node_ref":"0".repeat(64),"resolved_version":"1.0.0"}]}]);
    fs::write(reports.join("npm-bad-node.json"), malformed.to_string()).unwrap();
    let mut duplicate: Value = serde_json::from_slice(&fs::read(&original).unwrap()).unwrap();
    duplicate["run_id"] = serde_json::json!("npm-duplicate");
    let text = duplicate.to_string();
    let text = text.replacen("{", "{\"authority\":\"approved\",", 1);
    fs::write(reports.join("npm-duplicate.json"), text).unwrap();
    let native = fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| serde_json::from_slice::<Value>(&fs::read(p).unwrap()).unwrap())
        .find(|r| {
            r["component_count"] == 1
                && r["diagnostic_reason"] == "npm_database_and_policy_unverified"
        })
        .unwrap();
    let mut impossible = native;
    impossible["run_id"] = serde_json::json!("npm-fake-preparation");
    impossible["diagnostic_reason"] = serde_json::json!("npm_execution_context_missing");
    fs::write(
        reports.join("npm-fake-preparation.json"),
        impossible.to_string(),
    )
    .unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "work",
            "sync",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let sync: Value = serde_json::from_slice(&sync.stdout).unwrap();
    assert_eq!(sync["failed_reports"], 3, "{sync}");
    assert_eq!(
        fs::read_dir(fixture.0.join("codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[ignore = "requires explicit existing Node/npm11.16.0; native public command empty-lock observation"]
fn public_cve_command_runs_real_npm_without_claiming_delivery() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-cli-{}-native", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    fs::write(fixture.0.join("package.json"),r#"{"name":"cg-cli-fixture","version":"1.0.0","private":true,"scripts":{"audit":"npm audit"}}"#).unwrap();
    fs::write(fixture.0.join("package-lock.json"),r#"{"name":"cg-cli-fixture","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"cg-cli-fixture","version":"1.0.0"}}}"#).unwrap();
    for f in ["user.npmrc", "global.npmrc"] {
        fs::write(fixture.0.join(f), "").unwrap();
    }
    let user = fixture.0.join("user.npmrc");
    let global = fixture.0.join("global.npmrc");
    let node = std::env::var("CODEGUARD_NODE_BIN").unwrap();
    let entry = fs::canonicalize(std::env::var("CODEGUARD_NPM_ENTRY").unwrap()).unwrap();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            fixture.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "cve",
            "typescript",
            fixture.0.to_str().unwrap(),
            "--node-tool",
            &node,
            "--npm-entry",
            entry.to_str().unwrap(),
            "--npm-version",
            "11.16.0",
            "--userconfig",
            user.to_str().unwrap(),
            "--globalconfig",
            global.to_str().unwrap(),
            "--timeout",
            "90s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_coherent"], true, "{report}");
    assert_eq!(report["configuration"]["configuration"], "configured");
    assert_eq!(report["dependency_total"], 0);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["advisory_coverage"], "not_evaluated");
    assert!(!fixture.0.join("node_modules").exists());
    assert_eq!(report["workbench_status"], "synced_partial");
    assert_eq!(
        fs::read_dir(fixture.0.join("codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    let tasks: Vec<_> = fs::read_dir(fixture.0.join("codeguard/tasks"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let task = tasks[0].file_stem().unwrap().to_str().unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "task",
            "verify",
            task,
            fixture.0.to_str().unwrap(),
            "--node-tool",
            &node,
            "--npm-entry",
            entry.to_str().unwrap(),
            "--npm-version",
            "11.16.0",
            "--userconfig",
            user.to_str().unwrap(),
            "--globalconfig",
            global.to_str().unwrap(),
            "--timeout",
            "90s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        verified.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verified["event_persisted"], true, "{verified}");
    assert_eq!(verified["observation"], "still_blocked");
    assert_eq!(verified["native_scan"]["advisories"], serde_json::json!([]));
    assert!(!fixture.0.join("node_modules").exists());
}
