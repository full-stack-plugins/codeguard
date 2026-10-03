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
fn check_all_runs_each_npm_root_and_syncs_without_claiming_vulnerability_coverage() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-check-npm-{}", std::process::id()));
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
    fs::write(&node,format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else printf '%s\\n' \"$PWD\" >> '{}'; printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}'; fi\n",trace.display())).unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
    let paths: Vec<_> = ["npm.js", "user.npmrc", "global.npmrc"]
        .iter()
        .map(|n| {
            let p = fixture.0.join(n);
            fs::write(&p, "").unwrap();
            p
        })
        .collect();
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
    // 无显式上下文也自动记录准备阻塞，不启动原生命令。
    for _ in 0..2 {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
                fixture.0.to_str().unwrap(),
                "--jobs",
                "2",
                "--timeout",
                "10s",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(3));
        for scan in report["native_results"]["npm_cve"].as_array().unwrap() {
            assert_eq!(scan["feedback"]["reason"], "npm_execution_context_missing");
            assert_eq!(scan["feedback"]["local_coherent"], false);
            assert_eq!(scan["observation"]["advisories"], serde_json::json!([]));
            assert_eq!(scan["backlog_status"], "synced_partial", "{scan}");
        }
        assert_eq!(
            fs::read_dir(fixture.0.join(".codeguard/tasks"))
                .unwrap()
                .count(),
            2
        );
        assert!(!trace.exists());
    }
    for _ in 0..2 {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
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
                "--jobs",
                "2",
                "--timeout",
                "10s",
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
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["delivery_decision"], "incomplete");
        let scans = report["native_results"]["npm_cve"].as_array().unwrap();
        assert_eq!(scans.len(), 2);
        for scan in scans {
            assert_eq!(scan["feedback"]["local_coherent"], true, "{scan}");
            assert_eq!(scan["backlog_status"], "synced_partial", "{scan}");
            assert_eq!(scan["feedback"]["advisory_coverage"], "not_evaluated");
        }
        assert_eq!(report["execution_tasks"].as_array().unwrap().len(), 2);
        assert_eq!(
            fs::read_dir(fixture.0.join(".codeguard/tasks"))
                .unwrap()
                .count(),
            2
        );
    }
    let trace = fs::read_to_string(trace).unwrap();
    for name in ["a", "ab"] {
        assert_eq!(
            trace
                .lines()
                .filter(|l| *l == fixture.0.join(name).to_str().unwrap())
                .count(),
            2
        );
    }
    assert!(!fixture.0.join("a/.codeguard").exists());
    let native_args = [
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
    ];
    for (expected, tail) in [
        (2, vec!["--node-tool", node.to_str().unwrap()]),
        (3, vec!["--registry", "http://bad host"]),
    ] {
        let mut args = vec![
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ];
        args.extend(native_args);
        args.extend(tail);
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(expected));
    }
    let java = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", fixture.0.to_str().unwrap()])
        .args(native_args)
        .output()
        .unwrap();
    assert_eq!(java.status.code(), Some(2));
    let no_context = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let no_context: Value = serde_json::from_slice(&no_context.stdout).unwrap();
    let preparations = no_context["native_results"]["npm_cve"].as_array().unwrap();
    assert_eq!(preparations.len(), 2);
    for scan in preparations {
        assert_eq!(scan["feedback"]["reason"], "npm_execution_context_missing");
        assert_eq!(scan["feedback"]["local_coherent"], false);
        assert_eq!(scan["backlog_status"], "synced_partial");
        assert_eq!(scan["feedback"]["workbench"]["new_blockers"], 0);
    }
    // 一个根的原生错误不吞掉另一个根的结果，也不生成源码违规任务。
    fs::write(&node,"#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else case \"$PWD\" in */a) printf '%s' '{\"error\":{\"code\":\"EAUDITNOLOCK\"}}'; exit 1;; *) printf '%s' '{\"auditReportVersion\":2,\"vulnerabilities\":{},\"metadata\":{\"vulnerabilities\":{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0},\"dependencies\":{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}';; esac; fi\n").unwrap();
    let mixed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--jobs",
            "2",
            "--format",
            "json",
        ])
        .args(native_args)
        .output()
        .unwrap();
    let mixed: Value = serde_json::from_slice(&mixed.stdout).unwrap();
    let scans = mixed["native_results"]["npm_cve"].as_array().unwrap();
    assert_eq!(
        scans
            .iter()
            .filter(|r| r["feedback"]["local_coherent"] == true)
            .count(),
        1,
        "{mixed}"
    );
    assert_eq!(
        scans
            .iter()
            .filter(|r| r["feedback"]["local_coherent"] == false)
            .count(),
        1
    );
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        2
    );
    // 报告可读但npm异常退出：保留漏洞证据，执行节点必须仍为未完成。
    fs::write(
        fixture.0.join("a/package.json"),
        r#"{"scripts":{"audit":"npm audit"},"dependencies":{"fixture-pkg":"1.2.3"}}"#,
    )
    .unwrap();
    fs::write(
        fixture.0.join("a/package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{"dependencies":{"fixture-pkg":"1.2.3"}},"node_modules/fixture-pkg":{"version":"1.2.3"}}}"#,
    )
    .unwrap();
    let advisory = serde_json::json!({"auditReportVersion":2,"vulnerabilities":{"fixture-pkg":{"name":"fixture-pkg","severity":"high","isDirect":true,"via":[{"source":123,"name":"fixture-pkg","dependency":"fixture-pkg","title":"fixture","url":"https://example.invalid","severity":"high","range":"<2.0.0"}],"effects":[],"range":"<2.0.0","nodes":["node_modules/fixture-pkg"],"fixAvailable":false}},"metadata":{"vulnerabilities":{"info":0,"low":0,"moderate":0,"high":1,"critical":0,"total":1},"dependencies":{"prod":2,"dev":0,"optional":0,"peer":0,"peerOptional":0,"total":1}}});
    fs::write(&node, format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else case \"$PWD\" in */a) printf '%s' '{}'; exit 2;; *) printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}';; esac; fi\n", advisory)).unwrap();
    let partial = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .args(native_args)
        .output()
        .unwrap();
    let partial: Value = serde_json::from_slice(&partial.stdout).unwrap();
    let partial_scans = partial["native_results"]["npm_cve"].as_array().unwrap();
    assert!(
        partial_scans.iter().any(|scan| scan["feedback"]["reason"]
            == "npm_audit_execution_incomplete"
            && scan["feedback"]["findings"]
                .as_array()
                .is_some_and(|f| f.len() == 1)),
        "{partial}"
    );
    assert!(
        partial["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["status"] == "native_incomplete"),
        "{partial}"
    );
    fs::write(&node, format!("#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else case \"$PWD\" in */a) printf '%s' '{}'; /bin/sleep 0.2; /usr/bin/head -c 9000000 /dev/zero >&2; /bin/sleep 5;; *) printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}';; esac; fi\n", advisory)).unwrap();
    let limited = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--format",
            "json",
        ])
        .args(native_args)
        .output()
        .unwrap();
    let limited: Value = serde_json::from_slice(&limited.stdout).unwrap();
    assert!(
        limited["native_results"]["npm_cve"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |scan| scan["feedback"]["reason"] == "npm_audit_output_limit"
                    && scan["feedback"]["findings"]
                        .as_array()
                        .is_some_and(|f| f.len() == 1)
            ),
        "{limited}"
    );
    assert!(
        limited["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["status"] == "native_incomplete"),
        "{limited}"
    );
    fs::write(
        &node,
        "#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else /bin/sleep 5; fi\n",
    )
    .unwrap();
    let started = std::time::Instant::now();
    let timed = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            fixture.0.to_str().unwrap(),
            "--jobs",
            "1",
            "--timeout",
            "500ms",
            "--format",
            "json",
        ])
        .args(native_args)
        .output()
        .unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    let timed: Value = serde_json::from_slice(&timed.stdout).unwrap();
    assert_eq!(timed["delivery_decision"], "incomplete");
    assert!(
        timed["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|task| matches!(
                task["status"].as_str(),
                Some("deadline_exceeded" | "deadline_before_start")
            )),
        "{timed}"
    );
}

#[test]
#[ignore = "requires explicit existing Node/npm11.16.0; no install or package scripts"]
fn real_npm_audit_enters_check_all_workbench() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-check-npm-native-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let manifest = r#"{"name":"cg-native-check","version":"1.0.0","private":true,"scripts":{"audit":"npm audit"}}"#;
    let lock = r#"{"name":"cg-native-check","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"cg-native-check","version":"1.0.0"}}}"#;
    fs::write(fixture.0.join("package.json"), manifest).unwrap();
    fs::write(fixture.0.join("package-lock.json"), lock).unwrap();
    let user = fixture.0.join("user.npmrc");
    let global = fixture.0.join("global.npmrc");
    fs::write(&user, "").unwrap();
    fs::write(&global, "").unwrap();
    let node = std::env::var("CODEGUARD_NODE_BIN").unwrap();
    let entry = fs::canonicalize(std::env::var("CODEGUARD_NPM_ENTRY").unwrap()).unwrap();
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
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
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
            "--jobs",
            "1",
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
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.34.0");
    assert_eq!(
        report["native_results"]["npm_cve"][0]["feedback"]["local_coherent"], true,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["npm_cve"][0]["backlog_status"],
        "synced_partial"
    );
    assert_eq!(
        fs::read_dir(fixture.0.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        report["next"]["repair_brief"]["checker_id"],
        "node.npm.audit"
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("package.json")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("package-lock.json")).unwrap(),
        lock
    );
    assert!(!fixture.0.join("node_modules").exists());
}

#[test]
fn automatic_npm_preparation_does_not_initialize_a_workspace_or_enter_java_only_checks() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-check-npm-uninitialized-{}", std::process::id()));
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
    let run = |selection: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                selection,
                fixture.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let report = run("all");
    let scans = report["native_results"]["npm_cve"].as_array().unwrap();
    assert_eq!(scans.len(), 1);
    assert_eq!(
        scans[0]["feedback"]["reason"],
        "npm_execution_context_missing"
    );
    assert_eq!(
        scans[0]["feedback"]["workbench_status"],
        "workspace_not_initialized"
    );
    assert!(scans[0]["observation"].is_null());
    assert!(!fixture.0.join(".codeguard").exists());
    let report = run("java");
    assert_eq!(report["native_results"]["npm_cve"], serde_json::json!([]));
    assert!(!fixture.0.join(".codeguard").exists());
}
