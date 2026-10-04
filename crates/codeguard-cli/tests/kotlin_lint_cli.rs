#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn native_kotlin_syntax_feedback_retains_scope_without_granting_delivery() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-contract-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "fun f(x: ) = x\n").unwrap();
    let tool = root.join("kotlinc");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\nprintf '%s:1:10: error: [SYNTAX] Syntax error: Type expected.\\nfun f(x: ) = x\\n         ^\\n' \"$1\" >&2\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--kotlinc-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native"]["status"], "diagnostics_observed",
        "{report}"
    );
    assert_eq!(
        report["native"]["diagnostics"][0]["rule_id"], "kotlin.syntax",
        "{report}"
    );
    assert_eq!(
        report["native"]["diagnostics"][0]["column_byte"], 10,
        "{report}"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["coverage_proven"], false);
    let human = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--kotlinc-tool")
        .arg(&tool)
        .arg("--format=human")
        .output()
        .unwrap();
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(
        text.contains("kotlin.syntax") && text.contains(":1:10"),
        "{text}"
    );

    assert_eq!(fs::read_to_string(source).unwrap(), "fun f(x: ) = x\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kotlin_sigint_returns_cancelled_feedback_instead_of_incomplete_exit() {
    use std::time::{Duration, Instant};
    let root = std::env::temp_dir().join(format!("cg-kotlin-cancel-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let marker = root.join("started");
    let tool = root.join("kotlinc");
    fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\necho started > '{}'\n/bin/sleep 30\n",marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--kotlinc-tool")
        .arg(&tool)
        .arg("--format=json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists());
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["command_status"], "cancelled");
    assert_eq!(report["native"]["reason"], "request_cancelled");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn private_native_renderer_can_name_only_the_frozen_sample() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-relative-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "fun f(x: ) = x\n").unwrap();
    let tool = root.join("kotlinc");
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\nprintf 'Sample.kt:1:10: error: [SYNTAX] Type expected.\\nfun f(x: ) = x\\n         ^\\n' >&2\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--kotlinc-tool")
        .arg(&tool)
        .arg("--format=json")
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native"]["status"], "diagnostics_observed",
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "wasm-precheck")]
#[test]
fn absent_kotlin_native_tool_retains_hidden_wasm_incompleteness() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-fallback-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "fun f(x: ) = x\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--format=json")
        .env("PATH", &root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        report["syntax_precheck"]["precheck"]["truncated_files"], 1,
        "{report}"
    );
    assert_eq!(
        report["syntax_precheck"]["grammar_qualified"], false,
        "{report}"
    );
    assert_eq!(
        report["setup"]["native_confirmation_required"], true,
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kotlin_native_protocol_failures_and_context_never_become_syntax_success() {
    for (name, body, reason, syntax, context) in [
        (
            "semantic",
            "printf 'Sample.kt:1:10: error: [UNRESOLVED_REFERENCE] Unknown.\\n' >&2; exit 1",
            "kotlin_project_context_unresolved",
            0,
            1,
        ),
        (
            "mixed",
            "printf 'Sample.kt:1:10: error: [SYNTAX] Bad.\\nSample.kt:1:13: error: [UNRESOLVED_REFERENCE] Unknown.\\n' >&2; exit 1",
            "kotlin_project_context_unresolved",
            1,
            1,
        ),
        (
            "foreign",
            "printf 'Other.kt:1:10: error: [SYNTAX] Bad.\\n' >&2; exit 1",
            "kotlin_native_report_invalid",
            0,
            0,
        ),
        (
            "contradictory",
            "printf 'Sample.kt:1:10: error: [SYNTAX] Bad.\\n' >&2; exit 0",
            "kotlin_exit_diagnostics_inconsistent",
            0,
            0,
        ),
        (
            "silent_failure",
            "exit 1",
            "kotlin_exit_diagnostics_inconsistent",
            0,
            0,
        ),
        (
            "unknown",
            "echo 'No space left on device' >&2; exit 1",
            "kotlin_native_report_invalid",
            0,
            0,
        ),
        (
            "changed_input",
            "printf changed > \"$1\"; exit 0",
            "kotlin_input_or_launcher_changed_during_check",
            0,
            0,
        ),
        (
            "changed_launcher",
            "printf '#changed\\n' >> \"$0\"; exit 0",
            "kotlin_input_or_launcher_changed_during_check",
            0,
            0,
        ),
    ] {
        let root = std::env::temp_dir().join(format!("cg-kotlin-{name}-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("App.kt");
        fs::write(&source, "fun f(x: ) = x\n").unwrap();
        let tool = root.join("kotlinc");
        fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\n{body}\n")).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "kotlin"])
            .arg(&source)
            .arg("--kotlinc-tool")
            .arg(&tool)
            .arg("--format=json")
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(3), "{name}: {output:?}");
        assert_eq!(report["native"]["reason"], reason, "{name}: {report}");
        assert_eq!(
            report["native"]["diagnostics"].as_array().unwrap().len(),
            syntax,
            "{name}: {report}"
        );
        assert_eq!(
            report["setup"]["native_confirmation_required"], true,
            "{name}: {report}"
        );
        assert_eq!(
            report["native"]["context_diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            context,
            "{name}: {report}"
        );
        assert_eq!(
            report["syntax_precheck"],
            serde_json::Value::Null,
            "explicit tools must not be silently replaced"
        );
        assert_eq!(fs::read_to_string(source).unwrap(), "fun f(x: ) = x\n");
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn kotlin_invalid_arguments_and_scripts_do_not_execute_the_tool() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-arguments-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let tool = root.join("kotlinc");
    let marker = root.join("executed");
    fs::write(
        &tool,
        format!("#!/bin/sh\necho executed > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    for args in [
        vec!["--format", "bogus"],
        vec!["--timeout", "0s"],
        vec!["--timeout", "5"],
        vec!["--format=json", "--format=json"],
        vec!["--kotlinc-tool", "relative"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "kotlin"])
            .arg(&source)
            .args(args)
            .arg("--kotlinc-tool")
            .arg(&tool)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    let script = root.join("App.kts");
    fs::write(&script, "println(1)\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(script)
        .arg("--kotlinc-tool")
        .arg(tool)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn redirected_selected_tool_cannot_keep_native_completion() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("cg-kotlin-tool-redirect-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let selected = root.join("kotlinc");
    let first = root.join("first");
    let second = root.join("second");
    fs::write(&second, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&second, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&first,format!("#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; exit 0; fi\n/bin/ln -sf '{}' '{}'\nexit 0\n",second.display(),selected.display())).unwrap();
    fs::set_permissions(&first, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&first, &selected).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--kotlinc-tool")
        .arg(&selected)
        .arg("--format=json")
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["native"]["reason"], "kotlin_input_or_launcher_changed_during_check",
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_kotlin_path_tool_is_used_before_wasm_without_implicit_fallback() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-path-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let tool = root.join("kotlinc");
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = '-version' ]; then echo 'info: kotlinc-jvm 2.4.10 (JRE 21)' >&2; fi\nexit 0\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--format=json")
        .env("PATH", &root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["native"]["status"], "completed", "{report}");
    assert_eq!(report["tool_selection"]["source"], "path");
    assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kotlin_first_path_failure_and_explicit_bad_tool_do_not_select_alternatives() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-no-switch-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let first = root.join("first");
    let second = root.join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    let marker = root.join("second-used");
    for (dir, body) in [
        (
            &first,
            "#!/bin/sh\necho 'info: kotlinc-jvm 2.3.0 (JRE 21)' >&2\nexit 0\n".to_owned(),
        ),
        (
            &second,
            format!("#!/bin/sh\necho used > '{}'\nexit 0\n", marker.display()),
        ),
    ] {
        let tool = dir.join("kotlinc");
        fs::write(&tool, body).unwrap();
        fs::set_permissions(tool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let path = std::env::join_paths([&first, &second]).unwrap();
    for explicit in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.args(["lint", "kotlin"])
            .arg(&source)
            .arg("--format=json")
            .env("PATH", &path);
        if explicit {
            cmd.arg("--kotlinc-tool").arg(root.join("missing"));
        }
        let output = cmd.output().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(report["syntax_precheck"], serde_json::Value::Null);
        assert_eq!(
            report["native"]["reason"],
            if explicit {
                "kotlin_tool_unavailable_or_untrusted"
            } else {
                "kotlin_version_unverified_or_unsupported"
            },
            "{report}"
        );
    }
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(not(feature = "wasm-precheck"))]
#[test]
fn missing_native_and_wasm_backends_require_confirmation_instead_of_recommendation() {
    let root = std::env::temp_dir().join(format!("cg-kotlin-no-backend-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("App.kt");
    fs::write(&source, "class C {}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "kotlin"])
        .arg(&source)
        .arg("--format=json")
        .env("PATH", &root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["syntax_precheck"]["reason"],
        "wasm_feature_not_built"
    );
    assert_eq!(
        report["setup"]["native_confirmation_required"], true,
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}
