#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-check-all-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn check(&self, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
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
fn check_feedback_schema_cannot_encode_allow() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/check-feedback.schema.json")).unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.30.0");
    assert_eq!(
        schema["properties"]["export"]["properties"]["status"]["enum"],
        serde_json::json!(["not_requested", "saved", "failed"])
    );
    assert!(
        schema["properties"]["native_results"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "java_javadoc")
    );
    assert_eq!(
        schema["properties"]["native_results"]["properties"]["java_javadoc"]["oneOf"][1]["properties"]
            ["coverage_proven"]["const"],
        false
    );
    assert!(
        schema["properties"]["native_results"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "java_dependencies")
    );
    assert!(
        schema["properties"]["native_results"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "java_cve")
    );
    assert_eq!(
        schema["properties"]["native_results"]["properties"]["java_cve"]["oneOf"][1]["properties"]
            ["database_freshness"]["const"],
        "unverified"
    );
    assert_eq!(
        schema["properties"]["native_results"]["properties"]["java_dependencies"]["oneOf"][1]["properties"]
            ["coverage_proven"]["const"],
        false
    );
    let dependency = &schema["properties"]["native_results"]["properties"]["java_dependencies"]["oneOf"]
        [1]["properties"];
    assert_eq!(dependency["schema_version"]["const"], "0.2.0");
    assert_eq!(dependency["source_file_count"]["minimum"], 0);
    let native_graph = &dependency["probes"]["items"]["properties"]["observation"]["properties"];
    assert_eq!(native_graph["schema_version"]["const"], "0.2.0");
    assert!(
        native_graph["nodes"]["items"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "artifact_sha256")
    );
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "execution_budget")
    );
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "execution_tasks")
    );
    assert_eq!(
        schema["properties"]["delivery_decision"]["enum"],
        serde_json::json!(["incomplete", "not_evaluated"])
    );
    let candidates = &schema["properties"]["category_candidates"]["items"];
    assert!(
        candidates["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "checker_id")
    );
    assert!(
        candidates["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "next_action")
    );
    assert!(
        candidates["properties"]["status"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value != "allow" && value != "passed")
    );
    let javadoc = &schema["properties"]["native_results"]["properties"]["java_javadoc"]["oneOf"][1];
    assert_eq!(javadoc["properties"]["schema_version"]["const"], "0.3.0");
    assert!(
        javadoc["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "maven_multifile_probes")
    );
    let native =
        &javadoc["properties"]["maven_multifile_probes"]["items"]["properties"]["observation"];
    assert_eq!(
        native["properties"]["pom_mode"]["enum"],
        serde_json::json!(["unverified", "direct_pom_replay"])
    );
    assert_eq!(native["properties"]["coverage_proven"]["const"], false);
    assert_eq!(
        native["properties"]["delivery_decision"]["const"],
        "not_evaluated"
    );
    assert_eq!(
        schema["allOf"][0]["then"]["properties"]["delivery_decision"]["const"],
        "incomplete"
    );
    assert_eq!(
        schema["allOf"][0]["else"]["properties"]["delivery_decision"]["const"],
        "not_evaluated"
    );
    assert_eq!(
        schema["properties"]["exit_code"]["enum"],
        serde_json::json!([3, 130])
    );
    assert!(!schema.to_string().contains(".schema.json"));
}

#[test]
fn project_runtime_options_schema_rejects_quality_policy_fields() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/runtime-options.schema.json")).unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["schema_version"]["const"], "1.0");
    assert_eq!(
        schema["properties"]["document_type"]["const"],
        "codeguard_runtime_options"
    );
    assert!(schema["properties"].get("exclude").is_none());
    assert!(schema["properties"].get("rulepack").is_none());
    let expanded: Value = serde_json::from_str(include_str!(
        "../../../schemas/runtime-options-1.1.schema.json"
    ))
    .unwrap();
    assert_eq!(expanded["additionalProperties"], false);
    assert_eq!(expanded["properties"]["schema_version"]["const"], "1.1");
    assert_eq!(expanded["properties"]["jobs"]["minimum"], 1);
    assert_eq!(expanded["properties"]["jobs"]["maximum"], 64);
    assert!(expanded["properties"].get("exclude").is_none());
}

#[test]
fn jobs_limit_uses_cli_environment_project_and_builtin_priority() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir(project.0.join("src")).unwrap();
    fs::write(project.0.join("src/lib.rs"), "pub fn sample() {}\n").unwrap();
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.1","document_type":"codeguard_runtime_options","timeout":"30s","jobs":3}"#,
    )
    .unwrap();
    let (_, local) = project.check(&[]);
    assert_eq!(local["execution_budget"]["jobs_limit"], 3);
    assert_eq!(local["execution_budget"]["jobs_source"], "project_default");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", project.0.to_str().unwrap(), "--format=json"])
        .env("CODEGUARD_JOBS", "2")
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let environment: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(environment["execution_budget"]["jobs_limit"], 2);
    assert_eq!(
        environment["execution_budget"]["jobs_source"],
        "registered_environment"
    );
    let (_, cli) = project.check(&["--jobs", "1"]);
    assert_eq!(cli["execution_budget"]["jobs_limit"], 1);
    assert_eq!(cli["execution_budget"]["jobs_source"], "cli");

    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.1","document_type":"codeguard_runtime_options","timeout":"30s","jobs":0}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", project.0.to_str().unwrap(), "--format=json"])
        .env_remove("CODEGUARD_JOBS")
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let (_, overridden) = project.check(&["--jobs", "1", "--timeout", "30s"]);
    assert_eq!(overridden["execution_budget"]["jobs_source"], "cli");
    assert_eq!(overridden["execution_budget"]["source"], "cli");
}

#[test]
fn invalid_selected_jobs_are_rejected_before_native_execution() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let marker = project.0.join("native-started");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for jobs in ["0", "-1", "65", "2.5", "bogus"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
                project.0.to_str().unwrap(),
                "--jobs",
                jobs,
                "--ruff-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .env_remove("CODEGUARD_JOBS")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{jobs}");
        assert!(!marker.exists(), "{jobs}");
    }
}

#[test]
fn source_added_after_initial_discovery_is_an_explicit_scope_gap() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let new_source = project.0.join("late.py");
    let tool = project.0.join("ruff");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ruff 0.16.8\\n'; printf 'pass\\n' > '{}'; exit 0; fi\nexit 1\n",
            new_source.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();

    let (exit, report) = project.check(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert!(new_source.exists());
    assert!(
        report["discovery"]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| {
                row["id"] == "python"
                    && row["source_files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|path| path == "app.py")
                    && !row["source_files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|path| path == "late.py")
            })
    );
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| { reason == "project_scope_changed_during_check" })
    );

    fs::remove_file(&new_source).unwrap();
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&human.stdout).contains("重新发现项目并运行原工具复检"));
}

#[test]
fn same_source_path_changed_by_native_tool_is_an_explicit_input_gap() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    let source = project.0.join("app.py");
    fs::write(&source, "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = project.0.join("ruff");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ruff 0.16.8\\n'; printf 'import io\\n' > '{}'; exit 0; fi\nexit 1\n",
            source.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();

    let (exit, report) = project.check(&["--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert_eq!(fs::read(&source).unwrap(), b"import io\n");
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| { reason == "project_source_changed_during_check" })
    );

    fs::write(&source, "import os\n").unwrap();
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=human",
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&human.stdout).contains("核对并发编辑或检查器副作用"));
}

#[test]
fn checker_configuration_change_is_a_scope_gap_but_managed_records_are_not() {
    use std::os::unix::fs::PermissionsExt;

    for changes_configuration in [true, false] {
        let project = Project::new();
        fs::write(project.0.join("app.py"), "import os\n").unwrap();
        let configuration = project.0.join("ruff.toml");
        fs::write(&configuration, "[lint]\nselect = ['F401']\n").unwrap();
        let managed = project.0.join(".codeguard/state/native-observed");
        let action = if changes_configuration {
            format!(
                "printf '[lint]\\nselect = [\\\"E501\\\"]\\n' > '{}'",
                configuration.display()
            )
        } else {
            format!(
                "mkdir -p '{}'; touch '{}'",
                managed.parent().unwrap().display(),
                managed.display()
            )
        };
        let tool = project.0.join("ruff");
        fs::write(
            &tool,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ruff 0.16.8\\n'; {action}; exit 0; fi\nexit 1\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();

        let (exit, report) = project.check(&["--ruff-tool", tool.to_str().unwrap()]);
        assert_eq!(exit, 3);
        let changed = report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "project_scope_changed_during_check");
        assert_eq!(changed, changes_configuration);
        if !changes_configuration {
            assert!(
                !report["unresolved_conditions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|reason| reason == "project_source_changed_during_check")
            );
        }
    }
}

#[test]
fn oversized_source_snapshot_is_explicitly_incomplete() {
    let project = Project::new();
    let source = project.0.join("app.py");
    let file = fs::File::create(&source).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();

    let (exit, report) = project.check(&[]);
    assert_eq!(exit, 3);
    assert_eq!(report["delivery_decision"], "incomplete");
    assert!(
        report["unresolved_conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "project_source_snapshot_unavailable")
    );
}

#[test]
fn rust_only_project_keeps_categories_as_candidates_without_inventing_policy_obligations() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir(project.0.join("src")).unwrap();
    fs::write(project.0.join("src/lib.rs"), "pub fn sample() {}\n").unwrap();
    let (exit, report) = project.check(&[]);
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "check_feedback");
    assert_eq!(report["schema_version"], "0.30.0");
    assert_eq!(report["execution_budget"]["timeout_ms"], 1_800_000);
    assert_eq!(report["execution_budget"]["source"], "builtin_default");
    assert_eq!(
        report["execution_budget"]["jobs_limit"],
        std::thread::available_parallelism()
            .map(|count| count.get().clamp(1, 4))
            .unwrap_or(1)
    );
    assert_eq!(report["execution_budget"]["jobs_source"], "builtin_default");
    assert_eq!(report["execution_budget"]["native_task_count"], 4);
    assert_eq!(report["execution_budget"]["started_native_task_count"], 4);
    assert_eq!(
        report["execution_budget"]["enforcement"],
        "native_execution_only"
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    assert!(report["native_results"]["python_lint"].is_null());
    assert_eq!(report["execution_tasks"][0]["id"], "rust.cve");
    assert_eq!(report["execution_tasks"][0]["status"], "native_incomplete");
    assert_eq!(report["execution_tasks"][1]["id"], "rust.build");
    assert_eq!(report["execution_tasks"][2]["id"], "rust.comments");
    assert_eq!(report["execution_tasks"][3]["id"], "rust.lint");
    assert_eq!(
        report["native_results"]["rust_lint"]["reason"],
        "cargo_tool_not_selected"
    );
    assert!(report["required_obligations"].is_null());
    assert_eq!(report["obligation_status"], "unresolved");
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["language"] == "rust" && item["category"] == "lint")
    );
    assert!(!project.0.join(".codeguard").exists());
}

#[test]
fn configured_python_missing_tool_keeps_native_blocker_and_candidate_gaps_visible() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let (exit, report) = project.check(&["--ruff-tool", "/nonexistent/ruff"]);
    assert_eq!(exit, 3);
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(
        report["native_results"]["python_lint"]["files"][0]["reason"],
        "ruff_tool_not_found"
    );
    assert_eq!(report["execution_tasks"][0]["id"], "python.lint");
    assert_eq!(report["execution_tasks"][0]["status"], "native_incomplete");
    assert_eq!(report["execution_budget"]["native_task_count"], 1);
    assert_eq!(report["execution_budget"]["started_native_task_count"], 1);
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["language"] == "python"
                && item["category"] == "cve"
                && item["checker_id"] == "python.pip_audit"
                && item["status"] == "configuration_unresolved"
                && item["reason"] == "checker_configuration_unresolved")
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["language"] == "python"
                && item["category"] == "lint"
                && item["status"] == "native_incomplete")
    );
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--format=human",
            "--ruff-tool",
            "/nonexistent/ruff",
        ])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    let shown = String::from_utf8(human.stdout).unwrap();
    assert!(shown.contains("配置 python.ruff [lint]: configured"));
    assert!(shown.contains("候选 python / cve (python.pip_audit): configuration_unresolved"));
    assert!(shown.contains("python_dependency_input_not_found"));
    assert!(shown.contains("交付决策：incomplete"));
}

#[test]
fn initialized_check_all_preserves_scan_sync_next_chain() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    let (exit, report) = project.check(&[]);
    assert_eq!(exit, 3);
    assert_eq!(
        report["native_results"]["python_lint"]["backlog_status"],
        "synced_partial"
    );
    assert_eq!(
        report["native_results"]["python_lint"]["repair_brief_status"],
        "available"
    );
    assert_eq!(report["next"]["repair_brief"]["kind"], "blocker");
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn invalid_check_budget_is_rejected_before_native_execution() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let marker = project.0.join("native-started");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for budget in ["0ms", "-1s", "2days", "25h", "bogus"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
                project.0.to_str().unwrap(),
                "--timeout",
                budget,
                "--ruff-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{budget}");
        assert!(output.stdout.is_empty(), "{budget}");
        assert!(!marker.exists(), "{budget}");
    }
}

#[test]
fn registered_timeout_environment_is_used_but_cli_override_wins() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir(project.0.join("src")).unwrap();
    fs::write(project.0.join("src/lib.rs"), "pub fn sample() {}\n").unwrap();
    let invoke = |extra: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "all", project.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env("CODEGUARD_TIMEOUT", "250ms")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let environment = invoke(&[]);
    assert_eq!(environment["execution_budget"]["timeout_ms"], 250);
    assert_eq!(
        environment["execution_budget"]["source"],
        "registered_environment"
    );
    let overridden = invoke(&["--timeout", "2s"]);
    assert_eq!(overridden["execution_budget"]["timeout_ms"], 2000);
    assert_eq!(overridden["execution_budget"]["source"], "cli");
}

#[test]
fn versioned_project_runtime_default_has_lower_priority_than_env_and_cli() {
    let project = Project::new();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir(project.0.join("src")).unwrap();
    fs::write(project.0.join("src/lib.rs"), "pub fn sample() {}\n").unwrap();
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"250ms"}"#,
    )
    .unwrap();
    let (_, project_default) = project.check(&[]);
    assert_eq!(project_default["execution_budget"]["timeout_ms"], 250);
    assert_eq!(
        project_default["execution_budget"]["source"],
        "project_default"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", project.0.to_str().unwrap(), "--format=json"])
        .env("CODEGUARD_TIMEOUT", "1s")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let environment: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(environment["execution_budget"]["timeout_ms"], 1000);
    assert_eq!(
        environment["execution_budget"]["source"],
        "registered_environment"
    );
    let (_, cli) = project.check(&["--timeout", "2s", "--jobs", "1"]);
    assert_eq!(cli["execution_budget"]["timeout_ms"], 2000);
    assert_eq!(cli["execution_budget"]["source"], "cli");

    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"0ms"}"#,
    )
    .unwrap();
    let (_, cli) = project.check(&["--timeout", "2s", "--jobs", "1"]);
    assert_eq!(cli["execution_budget"]["source"], "cli");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", project.0.to_str().unwrap(), "--format=json"])
        .env("CODEGUARD_TIMEOUT", "1s")
        .env("CODEGUARD_JOBS", "2")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let environment: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        environment["execution_budget"]["source"],
        "registered_environment"
    );
}

#[test]
fn malformed_or_linked_project_runtime_default_is_rejected_before_native_execution() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    let marker = project.0.join("native-started");
    let tool = project.0.join("ruff");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = project.0.join(".codeguard/runtime.json");
    for content in [
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"0ms"}"#,
        r#"{"schema_version":"2.0","document_type":"codeguard_runtime_options","timeout":"1s"}"#,
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"1s","exclude":["**"]}"#,
    ] {
        fs::write(&runtime, content).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "check",
                "all",
                project.0.to_str().unwrap(),
                "--ruff-tool",
                tool.to_str().unwrap(),
                "--format=json",
            ])
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(!marker.exists());
    }
    fs::remove_file(&runtime).unwrap();
    std::os::unix::fs::symlink(project.0.join("app.py"), &runtime).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!marker.exists());
    fs::remove_file(&runtime).unwrap();
    fs::write(&runtime, format!("{{\"schema_version\":\"1.0\",\"document_type\":\"codeguard_runtime_options\",\"timeout\":\"1s\"}}{}", " ".repeat(4096))).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!marker.exists());
    fs::remove_file(&runtime).unwrap();
    fs::remove_dir(project.0.join(".codeguard")).unwrap();
    let alternate = project.0.join("elsewhere");
    fs::create_dir(&alternate).unwrap();
    fs::write(
        alternate.join("runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"1s"}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(&alternate, project.0.join(".codeguard")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--format=json",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!marker.exists());
}

#[test]
fn check_budget_expires_during_native_probe_without_reset_or_false_completion() {
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    let tool = project.0.join("ruff");
    fs::write(&tool, "#!/bin/sh\nsleep 2\nprintf 'ruff 0.16.8\\n'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let started = Instant::now();
    let (exit, report) =
        project.check(&["--timeout", "100ms", "--ruff-tool", tool.to_str().unwrap()]);
    assert_eq!(exit, 3);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(report["execution_budget"]["timeout_ms"], 100);
    assert_eq!(report["execution_budget"]["source"], "cli");
    assert_ne!(
        report["execution_tasks"][0]["status"],
        "native_observed_unverified"
    );
    assert_eq!(
        report["native_results"]["python_lint"]["local_scan_complete"],
        false
    );
    assert_eq!(
        report["native_results"]["python_lint"]["files"][0]["reason"],
        "request_deadline_exceeded"
    );
}

#[test]
fn check_all_cancelled_native_task_returns_130_and_keeps_discovery() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    use std::thread;

    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::create_dir(project.0.join("src")).unwrap();
    fs::write(
        project.0.join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    fs::write(
        project.0.join("Cargo.toml"),
        "[package]\nname='cancel-sample'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    let ready = project.0.join("native-ready");
    let rust_ready = project.0.join("rust-ready");
    let late = project.0.join("native-late-write");
    let armed = project.0.join("cancellation-armed");
    let tool = project.0.join("ruff");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\n(while [ ! -f '{}' ]; do sleep 0.01; done; sleep 0.6; /usr/bin/touch '{}') &\n/usr/bin/touch '{}'\nwait\n",
            armed.display(), late.display(), ready.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let cargo_tool = project.0.join("cargo-native");
    fs::write(
        &cargo_tool,
        format!(
            "#!/bin/sh\nsleep 2.2\nprintf '%s\\n' '{{\"reason\":\"compiler-message\",\"message\":{{\"level\":\"warning\",\"code\":{{\"code\":\"clippy::needless_return\"}},\"message\":\"unneeded return\",\"spans\":[{{\"file_name\":\"src/lib.rs\",\"line_start\":1,\"column_start\":1,\"is_primary\":true}}]}}}}' '{{\"reason\":\"build-finished\",\"success\":true}}'\n/usr/bin/touch '{}'\n",
            rust_ready.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&cargo_tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "check",
            "all",
            project.0.to_str().unwrap(),
            "--ruff-tool",
            tool.to_str().unwrap(),
            "--cargo-tool",
            cargo_tool.to_str().unwrap(),
            "--jobs",
            "2",
            "--timeout",
            "10s",
            "--format=json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while !(ready.exists() && rust_ready.exists()) && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(5));
    }
    if !(ready.exists() && rust_ready.exists()) {
        let running = child.try_wait().unwrap().is_none();
        if running {
            let _ = Command::new("/bin/kill")
                .args(["-INT", &child.id().to_string()])
                .output();
        }
        let output = child.wait_with_output().unwrap();
        panic!(
            "python_started={} rust_diagnostic_ready={} was_running={} exit={:?} stdout={} stderr={}",
            ready.exists(),
            rust_ready.exists(),
            running,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // Rust 输出准备完成后给收集器处理窗口；Python 始终等待独立取消屏障。
    thread::sleep(Duration::from_millis(200));
    fs::write(&armed, b"armed").unwrap();
    let interrupted = Command::new("/bin/kill")
        .args(["-INT", &child.id().to_string()])
        .output()
        .unwrap();
    assert!(interrupted.status.success());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["exit_code"], 130);
    assert_eq!(report["reason"], "request_cancelled");
    assert!(
        report["execution_budget"]["started_native_task_count"]
            .as_u64()
            .unwrap()
            >= 4
    );
    assert_eq!(report["execution_budget"]["jobs_limit"], 2);
    assert!(
        report["execution_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|task| task["status"] == "cancelled")
    );
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(
        report["native_results"]["rust_lint"]["findings"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert!(report["discovery"]["languages"].as_array().is_some());
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["language"] == "python"
                    && item["category"] == "lint"
                    && item["status"] == "native_incomplete"
            })
    );
    thread::sleep(Duration::from_millis(650));
    assert!(!late.exists(), "取消后原生子孙进程仍在写入");
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn original_ruff_finding_survives_check_all_partial_result() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new();
    fs::write(project.0.join("app.py"), "import os\n").unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['F401']\n").unwrap();
    fs::create_dir(project.0.join(".codeguard")).unwrap();
    fs::write(
        project.0.join(".codeguard/runtime.json"),
        r#"{"schema_version":"1.0","document_type":"codeguard_runtime_options","timeout":"30s"}"#,
    )
    .unwrap();
    let (exit, report) = project.check(&["--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert_eq!(report["execution_budget"]["timeout_ms"], 30_000);
    assert_eq!(report["execution_budget"]["source"], "project_default");
    assert_eq!(
        report["execution_tasks"][0]["status"],
        "native_observed_unverified"
    );
    let files = report["native_results"]["python_lint"]["files"]
        .as_array()
        .unwrap();
    assert!(files.iter().any(|file| {
        file["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "F401")
    }));
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["language"] == "python"
                    && item["category"] == "lint"
                    && item["status"] == "observed_unverified"
            })
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn native_ruff_d100_is_reported_as_python_comment_evidence() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new();
    fs::write(
        project.0.join("app.py"),
        "def calculate():\n    return 42\n",
    )
    .unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['D100']\n").unwrap();

    let (exit, report) = project.check(&["--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert!(
        report["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|file| file["findings"].as_array().into_iter().flatten())
            .any(|finding| {
                finding["rule_id"] == "D100"
                    && finding["rule_summary"] == "公共模块缺少文档字符串"
                    && finding["repair_hint"]["status"] == "bounded_repair_candidate"
            })
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["language"] == "python"
                    && candidate["category"] == "comments"
                    && candidate["checker_id"] == "python.ruff"
                    && candidate["status"] == "observed_unverified"
            })
    );
    assert_eq!(report["delivery_decision"], "incomplete");

    fs::write(
        project.0.join("app.py"),
        "\"\"\"Math helpers.\"\"\"\n\ndef calculate():\n    return 42\n",
    )
    .unwrap();
    let (recheck_exit, recheck) = project.check(&["--ruff-tool", &tool]);
    assert_eq!(recheck_exit, 3);
    assert!(
        !recheck["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["language"] == "python"
                    && candidate["category"] == "comments"
                    && candidate["status"] == "observed_unverified"
            })
    );
    assert_eq!(recheck["delivery_decision"], "incomplete");
}

#[test]
#[ignore = "requires pinned native Ruff executable via CODEGUARD_RUFF_BIN"]
fn native_ruff_d101_class_docstring_is_python_comment_evidence() {
    let tool = std::env::var("CODEGUARD_RUFF_BIN").unwrap();
    let project = Project::new();
    fs::write(
        project.0.join("app.py"),
        "\"\"\"Public module.\"\"\"\n\nclass PublicType:\n    pass\n",
    )
    .unwrap();
    fs::write(project.0.join("ruff.toml"), "[lint]\nselect = ['D101']\n").unwrap();

    let (exit, report) = project.check(&["--ruff-tool", &tool]);
    assert_eq!(exit, 3);
    assert!(
        report["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|file| file["findings"].as_array().into_iter().flatten())
            .any(|finding| {
                finding["rule_id"] == "D101"
                    && finding["rule_summary"] == "公共类缺少文档字符串"
                    && finding["repair_hint"]["status"] == "bounded_repair_candidate"
                    && finding["repair_hint"]["step"]
                        == "确认该公共类的职责，并为类补充准确的 docstring"
            })
    );
    assert!(
        report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["language"] == "python"
                    && candidate["category"] == "comments"
                    && candidate["checker_id"] == "python.ruff"
                    && candidate["status"] == "observed_unverified"
            })
    );
    assert_eq!(report["delivery_decision"], "incomplete");
}
