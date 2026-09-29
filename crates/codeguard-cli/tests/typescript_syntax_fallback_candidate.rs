#![cfg(all(feature = "wasm-precheck", unix))]

use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(std::path::PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "codeguard-syntax-fallback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn lint(&self, name: &str) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                self.0.join(name).to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_native_context_yields_scoped_suspected_observation_and_incomplete_gate() {
    let project = Project::new();
    fs::write(project.0.join("bad.ts"), "const x: number = ;\n").unwrap();
    let (status, report) = project.lint("bad.ts");
    assert_eq!(status, 3);
    assert_eq!(report["schema_version"], "0.3.0");
    assert_eq!(report["report_type"], "eslint_local_feedback");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["native"]["reason"], "explicit_context_missing");
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert!(
        !report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report["syntax_precheck"]["observations"][0]["classification"],
        "suspected"
    );
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn no_recovery_remains_incomplete_and_javascript_does_not_use_typescript_grammar() {
    let project = Project::new();
    fs::write(project.0.join("good.ts"), "const x: number = 1;\n").unwrap();
    let (_, report) = project.lint("good.ts");
    assert_eq!(report["syntax_precheck"]["status"], "incomplete");
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::write(project.0.join("app.js"), "const x = ;\n").unwrap();
    let (_, js_report) = project.lint("app.js");
    assert_eq!(js_report["schema_version"], "0.2.0");
    assert!(js_report.get("syntax_precheck").is_none());
}

#[test]
fn observed_project_local_eslint_candidate_is_not_treated_as_missing_native_lint() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const value: number = ;\n").unwrap();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(project.0.join("eslint.config.mjs"), "export default [];\n").unwrap();
    fs::create_dir_all(project.0.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        project.0.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("node_modules/eslint/bin/eslint.js"),
        "process.exit(99);\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("app.ts").to_str().unwrap(),
            "--format",
            "json",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["reason"], "eslint_node_runtime_unresolved");
    assert!(report.get("syntax_precheck").is_none());
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .contains("项目本地 ESLint 候选")
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn available_project_local_eslint_runs_before_wasm_and_preserves_native_finding() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    let source = project.0.join("app.ts");
    fs::write(&source, "debugger;\n").unwrap();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("eslint.config.cjs"),
        "module.exports = [];\n",
    )
    .unwrap();
    fs::create_dir_all(project.0.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        project.0.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("node_modules/eslint/bin/eslint.js"),
        "local entry fixture\n",
    )
    .unwrap();
    let native = serde_json::json!([{
        "filePath": source.canonicalize().unwrap(),
        "messages": [{"ruleId":"no-debugger","severity":2,"message":"private native text","line":1,"column":1}],
        "suppressedMessages":[],"errorCount":1,"warningCount":0,"fatalErrorCount":0,
        "fixableErrorCount":0,"fixableWarningCount":0
    }]);
    let node = project.0.join("node");
    fs::write(
        &node,
        format!(
            "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.0.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\nprintf '%s' '{}' > \"$report\"\nexit 1\n",
            native
        ),
    )
    .unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            source.to_str().unwrap(),
            "--format",
            "json",
        ])
        .env("PATH", &project.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_coherent"], true, "{report}");
    assert_eq!(report["findings"][0]["rule_id"], "no-debugger");
    assert!(report.get("syntax_precheck").is_none());
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private native text"));

    fs::write(&node, "#!/bin/sh\nprintf 'v10.0.1\\n'\n").unwrap();
    let mismatch = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            source.to_str().unwrap(),
            "--format",
            "json",
        ])
        .env("PATH", &project.0)
        .output()
        .unwrap();
    assert_eq!(mismatch.status.code(), Some(3));
    let mismatch: Value = serde_json::from_slice(&mismatch.stdout).unwrap();
    assert_eq!(mismatch["reason"], "eslint_version_mismatch");
    assert_eq!(mismatch["local_coherent"], false);
    assert!(mismatch.get("syntax_precheck").is_none());
}

#[test]
fn unresolved_node_updates_one_stable_environment_task_with_specific_guidance() {
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    let source = root.join("app.ts");
    fs::write(&source, "const value: number = 1;\n").unwrap();
    fs::write(
        root.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(root.join("eslint.config.cjs"), "module.exports = [];\n").unwrap();
    fs::create_dir_all(root.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        root.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    fs::write(root.join("node_modules/eslint/bin/eslint.js"), "fixture\n").unwrap();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            root.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));

    let mut task_id = None;
    for round in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                source.to_str().unwrap(),
                "--workspace",
                root.to_str().unwrap(),
                "--format",
                "json",
            ])
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["reason"], "eslint_node_runtime_unresolved");
        assert_eq!(report["workbench_status"], "synced_partial", "{report}");
        assert_eq!(report["workbench"]["new_blockers"], 1 - round);
        assert!(report["next_action"].as_str().unwrap().contains("Node"));
        assert!(
            report["next_action"]
                .as_str()
                .unwrap()
                .contains("不重复安装 ESLint")
        );
        let current = report["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        if let Some(previous) = &task_id {
            assert_eq!(&current, previous);
        }
        task_id = Some(current);
    }
}

#[test]
fn ambiguous_or_untrusted_local_eslint_is_a_setup_blocker_not_wasm_source_finding() {
    let project = Project::new();
    let root = project.0.canonicalize().unwrap();
    let source = root.join("bad.ts");
    fs::write(&source, "const value: number = ;\n").unwrap();
    fs::write(
        root.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        root.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    let entry = root.join("node_modules/eslint/bin/eslint.js");
    fs::write(&entry, "fixture\n").unwrap();
    fs::write(root.join("eslint.config.js"), "export default [];\n").unwrap();
    fs::write(root.join("eslint.config.mjs"), "export default [];\n").unwrap();
    let initialized = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            root.to_str().unwrap(),
            "--apply",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(initialized.status.code(), Some(3));
    let lint = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "typescript",
                source.to_str().unwrap(),
                "--workspace",
                root.to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let ambiguous = lint();
    assert_eq!(ambiguous["reason"], "eslint_config_selection_unresolved");
    assert!(ambiguous.get("syntax_precheck").is_none());
    assert!(ambiguous["findings"].as_array().unwrap().is_empty());
    assert_eq!(
        ambiguous["workbench_status"], "synced_partial",
        "{ambiguous}"
    );
    assert_eq!(ambiguous["workbench"]["new_blockers"], 1);
    assert!(
        ambiguous["next_action"]
            .as_str()
            .unwrap()
            .contains("flat config")
    );
    let task_id = ambiguous["workbench"]["next"]["repair_brief"]["task_id"].clone();

    fs::remove_file(root.join("eslint.config.mjs")).unwrap();
    fs::remove_file(&entry).unwrap();
    std::os::unix::fs::symlink(root.join("eslint.config.js"), &entry).unwrap();
    let untrusted = lint();
    assert_eq!(untrusted["reason"], "eslint_local_entry_untrusted");
    assert!(untrusted.get("syntax_precheck").is_none());
    assert!(untrusted["findings"].as_array().unwrap().is_empty());
    assert_eq!(
        untrusted["workbench_status"], "synced_partial",
        "{untrusted}"
    );
    assert_eq!(untrusted["workbench"]["new_blockers"], 0);
    assert_eq!(
        untrusted["workbench"]["next"]["repair_brief"]["task_id"],
        task_id
    );
    assert!(
        untrusted["next_action"]
            .as_str()
            .unwrap()
            .contains("路径不可信")
    );

    fs::remove_file(&entry).unwrap();
    fs::write(&entry, "fixture\n").unwrap();
    fs::write(root.join("node_modules/eslint/package.json"), b"{bad-json").unwrap();
    let invalid_identity = lint();
    assert_eq!(
        invalid_identity["reason"],
        "eslint_local_package_identity_invalid"
    );
    assert!(invalid_identity.get("syntax_precheck").is_none());
    assert_eq!(invalid_identity["workbench"]["new_blockers"], 0);
    assert_eq!(
        invalid_identity["workbench"]["next"]["repair_brief"]["task_id"],
        task_id
    );

    fs::write(
        root.join("package.json"),
        r#"{"devDependencies":{"eslint":"9.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        root.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"9.0.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    let unsupported = lint();
    assert_eq!(unsupported["reason"], "eslint_adapter_version_unsupported");
    assert!(unsupported.get("syntax_precheck").is_none());
    assert!(
        unsupported["next_action"]
            .as_str()
            .unwrap()
            .contains("补齐适配器")
    );
}

#[test]
fn configured_but_uninstalled_eslint_still_allows_candidate_precheck() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const value: number = ;\n").unwrap();
    fs::write(
        project.0.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.0.0"}}"#,
    )
    .unwrap();
    fs::write(project.0.join("eslint.config.mjs"), "export default [];\n").unwrap();

    let (status, report) = project.lint("app.ts");
    assert_eq!(status, 3);
    assert_eq!(report["native"]["status"], "not_run");
    assert!(report.get("syntax_precheck").is_some());
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn tsx_uses_its_own_grammar_instead_of_reporting_valid_jsx_as_suspect() {
    let project = Project::new();
    fs::write(
        project.0.join("component.tsx"),
        "const node = <div title=\"ok\">hello</div>;\n",
    )
    .unwrap();
    let (status, report) = project.lint("component.tsx");
    assert_eq!(status, 3);
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["syntax_precheck"]["language"], "tsx");
    assert_eq!(report["syntax_precheck"]["checked_files"], 1);
    assert_eq!(
        report["syntax_precheck"]["grammar_sha256"],
        "8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e"
    );
    assert!(
        report["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");

    fs::write(
        project.0.join("broken.tsx"),
        "const node = <div title=></div>;\n",
    )
    .unwrap();
    let (_, broken) = project.lint("broken.tsx");
    assert_eq!(broken["syntax_precheck"]["language"], "tsx");
    assert!(
        !broken["syntax_precheck"]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(broken["syntax_precheck"]["status"], "incomplete");
    assert!(broken["findings"].as_array().unwrap().is_empty());

    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/eslint-local-feedback-v0.4.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.4.0");
    assert_eq!(
        schema["properties"]["syntax_precheck"]["properties"]["language"]["const"],
        "tsx"
    );
    let actual: std::collections::BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let required: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(actual, required);
}

#[test]
fn candidate_feedback_schema_is_versioned_and_closed() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/eslint-local-feedback-v0.3.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.3.0");
    assert_eq!(schema["additionalProperties"], false);
    let project = Project::new();
    fs::write(project.0.join("good.ts"), "const x: number = 1;\n").unwrap();
    let (_, report) = project.lint("good.ts");
    let actual: std::collections::BTreeSet<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let required: std::collections::BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(actual, required);
}

#[test]
fn partial_native_context_or_symlink_does_not_trigger_candidate_worker() {
    let project = Project::new();
    fs::write(project.0.join("app.ts"), "const x: number = ;\n").unwrap();
    std::os::unix::fs::symlink(project.0.join("app.ts"), project.0.join("link.ts")).unwrap();
    let (_, linked) = project.lint("link.ts");
    assert_eq!(linked["schema_version"], "0.2.0");
    assert!(linked.get("syntax_precheck").is_none());

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("app.ts").to_str().unwrap(),
            "--config",
            project.0.join("eslint.config.js").to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let partial: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(partial["schema_version"], "0.2.0");
    assert!(partial.get("syntax_precheck").is_none());
}

#[test]
fn human_feedback_shows_suspected_location_and_native_confirmation_step() {
    let project = Project::new();
    fs::write(project.0.join("bad.ts"), "const x: number = ;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "typescript",
            project.0.join("bad.ts").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("内置语法初检"));
    assert!(text.contains("疑似语法"));
    assert!(text.contains("行 1，列"));
    assert!(text.contains("原生"));
}
