use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn go_lint_without_explicit_tool_remains_incomplete() {
    let root = std::env::temp_dir().join(format!("cg-go-lint-empty-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .args(["--format", "json"])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["language"], "go");
    assert_eq!(feedback["delivery_decision"], "not_evaluated");
    assert_eq!(feedback["report_type"], "go_lint_fallback_feedback");
    assert_eq!(feedback["native_report"]["native_status"], "incomplete");
    assert_eq!(feedback["native_report"]["reason"], "go_tool_not_selected");
    assert_eq!(feedback["native_tool_requirement"], "required");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unrelated_binary_and_invalid_timeout_cannot_report_a_clean_scan() {
    let root = std::env::temp_dir().join(format!(
        "codeguard-go-lint-wrong-tool-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/wrong-tool\n\ngo 1.23\n",
    )
    .unwrap();
    std::fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let wrong_tool = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .args(["--go-tool", "/bin/echo", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(wrong_tool.status.code(), Some(3));
    let feedback: Value = serde_json::from_slice(&wrong_tool.stdout).unwrap();
    assert_eq!(feedback["native_status"], "incomplete");
    assert_eq!(feedback["reason"], "go_tool_version_unvalidated");
    assert_eq!(feedback["findings"].as_array().unwrap().len(), 0);

    let bad_timeout = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go", "--timeout", "0s", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(bad_timeout.status.code(), Some(2));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_go_module_cannot_pass_with_an_unrelated_tool() {
    let root = std::env::temp_dir().join(format!(
        "codeguard-go-lint-nested-module-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("go.mod"), "module example.com/root\n\ngo 1.23\n").unwrap();
    std::fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    std::fs::write(
        root.join("nested/go.mod"),
        "module example.com/nested\n\ngo 1.23\n",
    )
    .unwrap();
    std::fs::write(
        root.join("nested/main.go"),
        "package main\nfunc main() {}\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .args(["--go-tool", "/bin/echo", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["native_status"], "incomplete");
    assert_eq!(feedback["reason"], "go_tool_version_unvalidated");
    assert_eq!(feedback["coverage_proven"], false);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn native_diagnostic_for_unobserved_source_does_not_become_a_finding() {
    let root = std::env::temp_dir().join(format!(
        "codeguard-go-lint-foreign-diagnostic-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("go.mod"), "module example.com/root\n\ngo 1.23\n").unwrap();
    std::fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let tool = root.join("go-fake");
    let report = format!(
        "{{\"example.com/root\":{{\"printf\":[{{\"posn\":\"{}:1:1\",\"message\":\"not observed\"}}]}}}}",
        root.canonicalize().unwrap().join("missing.go").display()
    );
    std::fs::write(
        &tool,
        format!(
            "#!/bin/sh\nif [ \"$1\" = version ]; then printf '%s\\n' 'go version go1.23.4 darwin/arm64'; else printf '%s\\n' '# example.com/root' '{report}' >&2; fi\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .arg("--go-tool")
        .arg(&tool)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["native_status"], "incomplete");
    assert_eq!(
        feedback["reason"],
        "go_vet_diagnostic_outside_discovered_sources"
    );
    assert_eq!(feedback["findings"].as_array().unwrap().len(), 0);

    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(
        root.join("nested/go.mod"),
        "module example.com/nested\n\ngo 1.23\n",
    )
    .unwrap();
    std::fs::write(
        root.join("nested/main.go"),
        "package main\nfunc main() {}\n",
    )
    .unwrap();
    let cross_module_report = format!(
        "{{\"example.com/root\":{{\"printf\":[{{\"posn\":\"{}:1:1\",\"message\":\"wrong module\"}}]}}}}",
        root.canonicalize()
            .unwrap()
            .join("nested/main.go")
            .display()
    );
    std::fs::write(
        &tool,
        format!(
            "#!/bin/sh\nif [ \"$1\" = version ]; then printf '%s\\n' 'go version go1.23.4 darwin/arm64'; else printf '%s\\n' '# example.com/root' '{cross_module_report}' >&2; fi\n"
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .arg("--go-tool")
        .arg(&tool)
        .args(["--format", "json"])
        .output()
        .unwrap();
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["native_status"], "incomplete");
    assert_eq!(feedback["modules_completed"], 0);
    assert_eq!(
        feedback["reason"],
        "go_vet_diagnostic_outside_discovered_sources"
    );
    assert_eq!(feedback["findings"].as_array().unwrap().len(), 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicitly supplied local Go 1.23.4 binary; no installation or network"]
fn real_go_lint_cli_reports_native_finding_clean_and_compile_error() {
    let go_tool = std::env::var("CODEGUARD_GO_TOOL").expect("指定 CODEGUARD_GO_TOOL");
    let root = std::env::temp_dir().join(format!("codeguard-go-lint-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/codeguard-go-cli\n\ngo 1.23\n",
    )
    .unwrap();
    let source = root.join("main.go");
    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "go"])
            .arg(&root)
            .args(["--go-tool", &go_tool, "--format", "json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    std::fs::write(
        &source,
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    let finding = run();
    assert_eq!(finding["native_status"], "findings_observed_unverified");
    assert_eq!(finding["findings"][0]["rule_id"], "printf");
    assert_eq!(finding["findings"][0]["path"], "main.go");
    assert_eq!(finding["delivery_decision"], "not_evaluated");
    assert_eq!(finding["coverage_proven"], false);
    let repeated = run();
    assert_eq!(
        finding["findings"][0]["finding_id"],
        repeated["findings"][0]["finding_id"]
    );
    std::fs::write(
        &source,
        "package main\n\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    let shifted = run();
    assert_eq!(
        finding["findings"][0]["finding_id"],
        shifted["findings"][0]["finding_id"]
    );
    assert_ne!(
        finding["findings"][0]["source_sha256"],
        shifted["findings"][0]["source_sha256"]
    );
    assert_eq!(shifted["findings"][0]["line"], 4);

    std::fs::write(
        &source,
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", 1) }\n",
    )
    .unwrap();
    let clean = run();
    assert_eq!(clean["native_status"], "clean_observed_unverified");
    assert_eq!(clean["findings"].as_array().unwrap().len(), 0);
    assert_eq!(clean["delivery_decision"], "not_evaluated");

    std::fs::write(&source, "package main\nfunc main() { missing() }\n").unwrap();
    let broken = run();
    assert_eq!(broken["native_status"], "incomplete");
    assert_eq!(broken["findings"].as_array().unwrap().len(), 0);

    std::fs::write(&source, "package main\nfunc main() {}\n").unwrap();
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(
        root.join("nested/go.mod"),
        "module example.com/codeguard-nested\n\ngo 1.23\n",
    )
    .unwrap();
    std::fs::write(
        root.join("nested/main.go"),
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    let nested = run();
    assert_eq!(nested["native_status"], "findings_observed_unverified");
    assert_eq!(nested["findings"][0]["path"], "nested/main.go");
    assert_eq!(nested["module_count"], 2);
    assert_eq!(nested["modules_completed"], 2);

    std::fs::write(
        &source,
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("nested/main.go"),
        "package main\nfunc main() { missing() }\n",
    )
    .unwrap();
    let partial = run();
    assert_eq!(partial["native_status"], "incomplete");
    assert_eq!(partial["findings"][0]["path"], "main.go");
    assert_eq!(partial["modules_completed"], 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn invalid_position_discards_all_findings_in_current_module() {
    let root = std::env::temp_dir().join(format!("codeguard-go-position-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("go.mod"), "module example.com/root\n\ngo 1.23\n").unwrap();
    std::fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let path = root.canonicalize().unwrap().join("main.go");
    let report = serde_json::json!({"example.com/root":{"printf":[
        {"posn":format!("{}:2:1", path.display()),"message":"valid location"},
        {"posn":format!("{}:200:1", path.display()),"message":"invalid location"}
    ]}});
    let tool = root.join("go-fake");
    std::fs::write(&tool, format!("#!/bin/sh\nif [ \"$1\" = version ]; then printf '%s\\n' 'go version go1.23.4 darwin/arm64'; else printf '%s\\n' '# example.com/root' '{report}' >&2; fi\n")).unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .arg("--go-tool")
        .arg(&tool)
        .args(["--format", "json"])
        .output()
        .unwrap();
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(feedback["reason"], "go_vet_source_location_invalid");
    assert_eq!(feedback["native_status"], "incomplete");
    assert!(feedback["findings"].as_array().unwrap().is_empty());
    assert_eq!(feedback["modules_completed"], 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn cancelled_go_vet_scan_returns_130_with_cancelled_status() {
    let root =
        std::env::temp_dir().join(format!("codeguard-go-lint-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("go.mod"), "module example.com/root\n\ngo 1.23\n").unwrap();
    std::fs::write(root.join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let tool = root.join("go-fake");
    std::fs::write(
        &tool,
        "#!/bin/sh\nif [ \"$1\" = version ]; then printf 'go version go1.23.4 darwin/arm64\\n'; exit 0; fi\nprintf started > \"$0.started\"\nexec /bin/sleep 30\n",
    )
    .unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "go"])
        .arg(&root)
        .arg("--go-tool")
        .arg(&tool)
        .args(["--format", "json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let marker = tool.with_extension("started");
    let wait_start = std::time::Instant::now();
    while !marker.exists() && wait_start.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(marker.exists(), "原生 vet 扫描必须先启动");
    let kill = Command::new("/bin/kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(kill.success(), "SIGINT 必须送达 CLI 进程");
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "取消必须返回 130 而不是普通未完成 3；stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    let feedback: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(feedback["command_status"], "cancelled", "{feedback}");
    assert_eq!(feedback["reason"], "request_cancelled", "{feedback}");
    assert_eq!(feedback["native_status"], "incomplete", "{feedback}");
    assert!(feedback["findings"].as_array().unwrap().is_empty());
    assert_eq!(feedback["delivery_decision"], "not_evaluated");
    std::fs::remove_dir_all(root).unwrap();
}
