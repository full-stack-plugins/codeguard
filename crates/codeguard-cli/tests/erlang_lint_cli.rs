#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-erlang-lint-{name}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("sample.erl"), "-module(sample).\nf() -> ok\n").unwrap();
    root
}

fn cli(root: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command
        .args(["lint", "erlang"])
        .arg(root.join("sample.erl"));
    command.args(extra).arg("--format=json");
    command.env_remove("CODEGUARD_TIMEOUT").env("PATH", "");
    command.output().unwrap()
}

fn fake(root: &Path, version: &str, output: &str, code: i32) -> std::path::PathBuf {
    let tool = root.join("erl");
    fs::write(&tool, format!(
        "#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' '{version}';; *) /bin/cat >/dev/null; printf '%s\\n' '{output}'; exit {code};; esac\n"
    )).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    tool
}

#[test]
fn explicit_native_diagnostic_precedes_wasm_and_is_not_a_delivery_pass() {
    let root = fixture("native");
    let tool = fake(
        &root,
        "OTP 28",
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[{"line":2,"column":8,"rule_id":"erlang.syntax.error"}]}"#,
        0,
    );
    let result = cli(&root, &["--erl-tool", tool.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["report_type"], "erlang_lint_feedback");
    assert_eq!(report["native"]["status"], "diagnostics_observed");
    assert_eq!(report["native"]["diagnostics"][0]["line"], 2);
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["coverage_proven"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn human_feedback_states_the_native_result_before_project_gaps() {
    let root = fixture("human");
    let tool = fake(
        &root,
        "OTP 28",
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}"#,
        0,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "erlang"])
        .arg(root.join("sample.erl"))
        .arg("--erl-tool")
        .arg(&tool)
        .args(["--format", "human"])
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Erlang 单文件原生语法未见错误"), "{text}");
    assert!(text.contains("完整项目检查"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_native_failure_keeps_the_blocker_without_wasm_or_path_fallback() {
    let root = fixture("failure");
    let tool = fake(&root, "OTP 28", "malformed report", 1);
    let result = cli(&root, &["--erl-tool", tool.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native"]["status"], "incomplete");
    assert_eq!(report["native"]["diagnostics"], serde_json::json!([]));
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_options_are_rejected_before_a_native_process_starts() {
    let root = fixture("args");
    let tool = root.join("erl");
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\n/usr/bin/touch '{}'\n",
            root.join("ran").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for extra in [
        vec!["--erl-tool", tool.to_str().unwrap(), "--timeout", "0s"],
        vec![
            "--erl-tool",
            tool.to_str().unwrap(),
            "--erl-tool",
            tool.to_str().unwrap(),
        ],
        vec!["--unknown"],
    ] {
        assert_eq!(cli(&root, &extra).status.code(), Some(2));
        assert!(!root.join("ran").exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn zero_diagnostics_with_preprocessing_remains_unresolved() {
    let root = fixture("preprocessor");
    let tool = fake(
        &root,
        "OTP 28",
        r#"{"schema_version":"0.1.0","forms":2,"preprocessing":true,"diagnostics_truncated":false,"diagnostics":[]}"#,
        0,
    );
    let result = cli(&root, &["--erl-tool", tool.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native"]["status"], "incomplete");
    assert_eq!(
        report["native"]["reason"],
        "erlang_preprocessing_unresolved"
    );
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_native_identity_or_positions_cannot_create_diagnostics() {
    let root = fixture("invalid-report");
    for native in [
        r#"{"schema_version":"2.0.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}"#,
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[{"line":9,"column":1,"rule_id":"erlang.syntax.error"}]}"#,
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[{"line":2,"column":99,"rule_id":"erlang.syntax.error"}]}"#,
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[],"diagnostics":[]}"#,
    ] {
        let tool = fake(&root, "OTP 28", native, 0);
        let result = cli(&root, &["--erl-tool", tool.to_str().unwrap()]);
        assert_eq!(result.status.code(), Some(3));
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["native"]["status"], "incomplete");
        assert_eq!(report["native"]["reason"], "erlang_syntax_report_invalid");
        assert_eq!(report["native"]["diagnostics"], serde_json::json!([]));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsupported_version_or_relative_tool_does_not_start_a_fallback() {
    let root = fixture("unsupported");
    let tool = fake(
        &root,
        "OTP 29",
        r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}"#,
        0,
    );
    for input in [tool.to_str().unwrap(), "erl"] {
        let result = cli(&root, &["--erl-tool", input]);
        assert_eq!(result.status.code(), Some(3));
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["native"]["status"], "incomplete");
        assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
        assert_eq!(report["native"]["diagnostics"], serde_json::json!([]));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_or_tool_mutation_invalidates_an_otherwise_completed_observation() {
    let root = fixture("mutation");
    let clean = r#"{"schema_version":"0.1.0","forms":1,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}"#;
    for (action, reason) in [
        (
            format!(
                "printf '%s\\n' '-module(changed).' > '{}'",
                root.join("sample.erl").display()
            ),
            "erlang_source_changed_during_check",
        ),
        (
            "printf '%s\\n' '# changed' >> \"$0\"".into(),
            "erlang_tool_changed_during_check",
        ),
    ] {
        let tool = root.join("erl");
        fs::write(&tool,format!("#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' 'OTP 28';; *) /bin/cat >/dev/null; {action}; printf '%s\\n' '{clean}';; esac\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let result = cli(&root, &["--erl-tool", tool.to_str().unwrap()]);
        assert_eq!(result.status.code(), Some(3));
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["native"]["status"], "incomplete");
        assert_eq!(report["native"]["reason"], reason);
        assert_eq!(report["native"]["diagnostics"], serde_json::json!([]));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_timeout_keeps_a_specific_execution_blocker() {
    let root = fixture("timeout");
    let tool = root.join("erl");
    fs::write(&tool,"#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' 'OTP 28';; *) exec /bin/sleep 5;; esac\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let started = std::time::Instant::now();
    let result = cli(
        &root,
        &["--erl-tool", tool.to_str().unwrap(), "--timeout", "300ms"],
    );
    assert_eq!(result.status.code(), Some(3));
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native"]["reason"], "request_deadline_exceeded");
    assert_eq!(report["execution_budget"]["source"], "cli");
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ctrl_c_preserves_cancellation_and_reaps_native_descendants() {
    let root = fixture("cancel");
    let ready = root.join("ready");
    let escaped = root.join("escaped");
    let tool = root.join("erl");
    fs::write(&tool,format!("#!/bin/sh\ncase \"$*\" in *system_info*) printf '%s\\n' 'OTP 28';; *) /usr/bin/touch '{}'; /bin/sleep 2; /usr/bin/touch '{}';; esac\n",ready.display(),escaped.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "erlang"])
        .arg(root.join("sample.erl"))
        .arg("--erl-tool")
        .arg(&tool)
        .arg("--format=json")
        .env_remove("CODEGUARD_TIMEOUT")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(ready.exists(), "native process did not start");
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(130), "{result:?}");
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native"]["reason"], "request_cancelled");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert!(!escaped.exists(), "native descendant escaped cancellation");
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing OTP 28 erl via CODEGUARD_ERL_BIN"]
fn actual_otp_parser_keeps_preprocessing_unknown_and_does_not_execute_project_startup() {
    let erl = std::env::var("CODEGUARD_ERL_BIN").expect("explicit existing OTP 28 erl");
    let root = fixture("real-scope");
    let marker = root.join("unexpected-execution");
    fs::write(
        root.join(".erlang"),
        format!(
            "file:write_file(\"{}\", <<\"executed\">>).\n",
            marker.display()
        ),
    )
    .unwrap();
    let samples = [
        ("-module(sample).\nf() -> ok.", "completed"),
        ("-module(sample).\nf() -> '中文'.\n", "completed"),
        (
            "-module(sample).\n-define(X, 1).\nf() -> ?X.\n",
            "incomplete",
        ),
        ("-module(sample).\nf() -> ?EXTERNAL.\n", "incomplete"),
        (
            "-module(sample).\nf() -> ?EXTERNAL \"text\".\n",
            "incomplete",
        ),
        (
            "-module(sample).\n-ifdef(FLAG).\nf() -> .\n-endif.\n",
            "incomplete",
        ),
        (
            "-module(sample).\n-compile({parse_transform, nonexistent}).\nf() -> ok.\n",
            "completed",
        ),
    ];
    for (source, status) in samples {
        fs::write(root.join("sample.erl"), source).unwrap();
        let result = cli(&root, &["--erl-tool", &erl]);
        assert_eq!(result.status.code(), Some(3), "{result:?}");
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["native"]["status"], status, "{report}");
        if status == "incomplete" {
            assert_eq!(
                report["native"]["reason"],
                "erlang_preprocessing_unresolved"
            );
        }
        assert_eq!(fs::read_to_string(root.join("sample.erl")).unwrap(), source);
        assert!(!marker.exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn missing_native_tool_runs_the_pinned_candidate_and_preserves_its_limit() {
    let root = fixture("missing");
    let result = cli(&root, &[]);
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["syntax_precheck"]["grammar_qualified"], false);
    assert_eq!(
        report["syntax_precheck"]["recoveries"],
        serde_json::json!([])
    );
    assert!(
        report["known_limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v
                .as_str()
                .unwrap()
                .contains("missing final function period"))
    );
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .contains("--erl-tool")
    );
    fs::remove_dir_all(root).unwrap();
}
