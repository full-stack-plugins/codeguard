#![cfg(unix)]
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit existing Node and ESLint 10.11.0; native distinct project contexts"]
fn native_monorepo_uses_original_child_configuration_without_applying_root_rule() {
    use std::os::unix::fs::DirBuilderExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-native-map-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let project = Project(root);
    std::fs::create_dir(project.0.join("child")).unwrap();
    for file in ["app.js", "child/app.js"] {
        std::fs::write(project.0.join(file), "debugger;").unwrap();
    }
    let root_config = project.0.join("eslint.config.cjs");
    std::fs::write(
        &root_config,
        "module.exports=[{rules:{'no-debugger':'error'}}];",
    )
    .unwrap();
    std::fs::write(
        project.0.join("child/eslint.config.cjs"),
        "module.exports=[{rules:{'no-debugger':'off'}}];",
    )
    .unwrap();
    let map = project.0.join("contexts.json");
    std::fs::write(&map,json!({"schema_version":"1.0","projects":[{"root":"child","config":"child/eslint.config.cjs","cwd":"child"}]}).to_string()).unwrap();
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
            root_config.to_str().unwrap(),
            "--cwd",
            project.0.to_str().unwrap(),
            "--config-map",
            map.to_str().unwrap(),
            "--timeout",
            "240s",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["input_stable"], true, "{report}");
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 4);
    for (path, count, scope) in [
        ("app.js", 1, "."),
        ("child/app.js", 0, "child"),
        ("child/eslint.config.cjs", 0, "child"),
        ("eslint.config.cjs", 0, "."),
    ] {
        let file = files.iter().find(|file| file["path"] == path).unwrap();
        assert_eq!(file["feedback"]["local_coherent"], true, "{file}");
        assert_eq!(
            file["feedback"]["findings"].as_array().unwrap().len(),
            count
        );
        assert_eq!(file["configuration_scope"], scope);
    }
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
#[test]
fn explicit_project_map_uses_component_boundaries_and_rejects_ambiguous_contexts() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-map-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let project = Project(root);
    for dir in ["packages/a", "packages/ab"] {
        std::fs::create_dir_all(project.0.join(dir)).unwrap();
    }
    for file in ["packages/a/app.js", "packages/ab/app.js"] {
        std::fs::write(project.0.join(file), "debugger;").unwrap();
    }
    let config = project.0.join("eslint.config.cjs");
    std::fs::write(&config, "fixture").unwrap();
    std::fs::write(project.0.join("packages/a/eslint.config.cjs"), "fixture").unwrap();
    let entry = project.0.join("entry.txt");
    std::fs::write(&entry, "fixture").unwrap();
    let node = project.0.join("node");
    let trace = project.0.join("invocations.txt");
    std::fs::write(&node,format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.11.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --config ]; then shift; config=$1; fi; if [ \"$1\" = --output-file ]; then shift; report=$1; fi; source=$1; shift; done\nprintf '%s|%s|%s\\n' \"$source\" \"$config\" \"$PWD\" >> '{}'\nprintf '[{{\"filePath\":\"%s\",\"messages\":[],\"suppressedMessages\":[],\"errorCount\":0,\"warningCount\":0,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}}]' \"$source\" > \"$report\"\nexit 0\n",trace.display())).unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o700)).unwrap();
    let map = project.0.join("contexts.json");
    let context =
        json!({"root":"packages/a","config":"packages/a/eslint.config.cjs","cwd":"packages/a"});
    std::fs::write(
        &map,
        json!({"schema_version":"1.0","projects":[{"root":".","config":"eslint.config.cjs","cwd":"."},context]}).to_string(),
    )
    .unwrap();
    let scan = || {
        Command::new(env!("CARGO_BIN_EXE_codeguard"))
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
                "--config-map",
                map.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap()
    };
    let output = scan();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["input_stable"], true, "{report}");
    let trace_text = std::fs::read_to_string(&trace).unwrap();
    assert!(trace_text.lines().any(|line| line
        == format!(
            "{}|{}|{}",
            project.0.join("packages/a/app.js").display(),
            project.0.join("packages/a/eslint.config.cjs").display(),
            project.0.join("packages/a").display()
        )));
    assert!(trace_text.lines().any(|line| line
        == format!(
            "{}|{}|{}",
            project.0.join("packages/ab/app.js").display(),
            config.display(),
            project.0.display()
        )));
    for invalid in [
        json!({"schema_version":"1.0","projects":[context.clone(),context.clone()]}),
        json!({"schema_version":"1.0","projects":[{"root":"../outside","config":"eslint.config.cjs","cwd":"."}]}),
        json!({"schema_version":"1.0","projects":[context],"approved":true}),
    ] {
        std::fs::write(&map, invalid.to_string()).unwrap();
        let before = std::fs::read(&trace).unwrap();
        let output = scan();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["reason"], "eslint_config_map_invalid", "{report}");
        assert_eq!(
            std::fs::read(&trace).unwrap(),
            before,
            "invalid map launched native scan"
        );
    }
    std::fs::write(&map,json!({"schema_version":"1.0","projects":[{"root":"packages/a","config":"packages/a/eslint.config.cjs","cwd":"packages/a"}]}).to_string()).unwrap();
    let script = std::fs::read_to_string(&node).unwrap().replace(
        "printf '[",
        &format!("printf changed > '{}'\nprintf '[", map.display()),
    );
    std::fs::write(&node, script).unwrap();
    let output = scan();
    let changed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        changed["reason"], "eslint_directory_input_changed",
        "{changed}"
    );
    assert_eq!(changed["input_stable"], false);
    assert_eq!(changed["files"].as_array().unwrap().len(), 1);
    assert_eq!(changed["unexecuted_files"].as_array().unwrap().len(), 3);
}
