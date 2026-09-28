#![cfg(unix)]
use serde_json::Value;
use std::{path::PathBuf, process::Command};
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0; native directory and configuration source"]
fn native_directory_preserves_individual_file_results_and_checks_original_config_source() {
    use std::os::unix::fs::DirBuilderExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-native-directory-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let project = Project(root);
    std::fs::create_dir(project.0.join("src")).unwrap();
    std::fs::write(project.0.join("src/bad.js"), "debugger;").unwrap();
    std::fs::write(project.0.join("src/clean.js"), "console.log('ok');").unwrap();
    let config = project.0.join("eslint.config.cjs");
    std::fs::write(&config, "module.exports=[{rules:{'no-debugger':'error'}}];").unwrap();
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.to_str().unwrap(),
            "--node-tool",
            std::path::Path::new(&node).to_str().unwrap(),
            "--eslint-entry",
            std::path::Path::new(&entry).to_str().unwrap(),
            "--eslint-version",
            "10.11.0",
            "--config",
            config.to_str().unwrap(),
            "--cwd",
            project.0.to_str().unwrap(),
            "--timeout",
            "240s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["report_type"], "eslint_directory_feedback");
    assert_eq!(report["input_stable"], true, "{report}");
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);
    for (path, count) in [
        ("src/bad.js", 1),
        ("src/clean.js", 0),
        ("eslint.config.cjs", 0),
    ] {
        let file = files.iter().find(|file| file["path"] == path).unwrap();
        assert_eq!(file["feedback"]["local_coherent"], true, "{file}");
        assert_eq!(
            file["feedback"]["findings"].as_array().unwrap().len(),
            count
        );
    }
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
#[test]
fn directory_scans_each_file_and_reuses_stable_tasks_without_claiming_full_coverage() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-directory-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let project = Project(root);
    for folder in ["a", "b", "codeguard/user", "node_modules/vendor"] {
        std::fs::create_dir_all(project.0.join(folder)).unwrap();
    }
    for file in [
        "a/app.js",
        "b/app.js",
        "codeguard/user/custom.js",
        "node_modules/vendor/ignored.js",
    ] {
        std::fs::write(project.0.join(file), "debugger;").unwrap();
    }
    let config = project.0.join("eslint.config.cjs");
    std::fs::write(&config, "fixture").unwrap();
    let entry = project.0.join("eslint-entry.txt");
    std::fs::write(&entry, "fixture").unwrap();
    let node = project.0.join("node");
    std::fs::write(&node, "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; source=$1; shift; done\nprintf '[{\"filePath\":\"%s\",\"messages\":[{\"ruleId\":\"no-debugger\",\"severity\":2,\"message\":\"fixture\",\"line\":1,\"column\":1}],\"suppressedMessages\":[],\"errorCount\":1,\"warningCount\":0,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"\nexit 1\n").unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let scan = |timeout: Option<&str>| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                project.0.to_str().unwrap(),
                "--node-tool",
                node.to_str().unwrap(),
                "--eslint-entry",
                entry.to_str().unwrap(),
                "--eslint-version",
                "10.11.0",
                "--config",
                config.to_str().unwrap(),
                "--cwd",
                project.0.to_str().unwrap(),
                "--workspace",
                project.0.to_str().unwrap(),
                "--format",
                "json",
            ])
            .args(timeout.map(|v| vec!["--timeout", v]).unwrap_or_default())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    for round in 0..2 {
        let report = scan(None);
        assert_eq!(
            report["report_type"], "eslint_directory_feedback",
            "{report}"
        );
        assert_eq!(report["files"].as_array().unwrap().len(), 4);
        assert_eq!(report["input_stable"], true);
        assert_eq!(report["coverage_proven"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        let scanned: Vec<_> = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap())
            .collect();
        assert!(scanned.contains(&"codeguard/user/custom.js"));
        assert!(scanned.contains(&"a/app.js") && scanned.contains(&"b/app.js"));
        let added: u64 = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["feedback"]["workbench"]["new_findings"].as_u64())
            .sum();
        assert_eq!(added, if round == 0 { 4 } else { 0 }, "{report}");
    }
    let script = std::fs::read_to_string(&node).unwrap().replace(
        "printf '[",
        "case \"$source\" in */b/app.js) exit 2;; esac\nprintf '[",
    );
    std::fs::write(&node, script).unwrap();
    let failed = scan(None);
    assert_eq!(failed["status"], "incomplete", "{failed}");
    assert_eq!(failed["reason"], "eslint_directory_file_incomplete");
    let files = failed["files"].as_array().unwrap();
    assert_eq!(
        files.iter().find(|f| f["path"] == "b/app.js").unwrap()["feedback"]["local_coherent"],
        false
    );
    assert_eq!(
        files.iter().find(|f| f["path"] == "a/app.js").unwrap()["feedback"]["local_coherent"],
        true
    );
    std::os::unix::fs::symlink(project.0.join("a/app.js"), project.0.join("linked.js")).unwrap();
    let linked = scan(None);
    assert!(
        linked["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["path"] == "linked.js" && s["reason"] == "symbolic_link_not_followed")
    );
    let slow = std::fs::read_to_string(&node).unwrap().replace(
        "then printf 'v10.11.0",
        "then /bin/sleep 1; printf 'v10.11.0",
    );
    std::fs::write(&node, slow).unwrap();
    let timed = scan(Some("100ms"));
    assert_eq!(timed["reason"], "request_deadline_exceeded", "{timed}");
    assert_eq!(timed["status"], "incomplete");
    assert_eq!(
        timed["unexecuted_files"].as_array().unwrap().len(),
        3,
        "{timed}"
    );
    let normal = std::fs::read_to_string(&node)
        .unwrap()
        .replace("then /bin/sleep 1; printf", "then printf")
        .replace("case \"$source\" in */b/app.js) exit 2;; esac\n", "");
    std::fs::write(&node, normal).unwrap();
    std::fs::rename(
        project.0.join(".codeguard/workspace.json"),
        project.0.join(".codeguard/workspace.backup.json"),
    )
    .unwrap();
    let disconnected = scan(None);
    assert_eq!(disconnected["status"], "incomplete", "{disconnected}");
    assert_eq!(
        disconnected["reason"],
        "eslint_directory_workbench_incomplete"
    );
}
