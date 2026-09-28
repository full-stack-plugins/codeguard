use codeguard_adapters::EslintDiagnostic;
use codeguard_adapters::project_eslint_findings;
fn finding(line: u32, column: u32) -> EslintDiagnostic {
    EslintDiagnostic {
        path: "/fixture/app.js".into(),
        rule_id: "no-debugger".into(),
        severity: 2,
        line,
        column,
        message: "native sensitive diagnostic".into(),
    }
}
#[test]
fn inserted_lines_and_message_changes_do_not_duplicate_the_same_problem() {
    let first = project_eslint_findings(
        "src/app.js",
        "/fixture/app.js",
        b"debugger;\n",
        &[finding(1, 1)],
    )
    .unwrap();
    let mut observed = finding(3, 1);
    observed.message = "different native wording".into();
    let moved = project_eslint_findings(
        "src/app.js",
        "/fixture/app.js",
        b"\n\ndebugger;\n",
        &[observed],
    )
    .unwrap();
    assert_eq!(first[0]["finding_id"], moved[0]["finding_id"]);
    assert_ne!(first[0]["source_sha256"], moved[0]["source_sha256"]);
    assert!(!first[0].to_string().contains("native sensitive diagnostic"));
}
#[test]
fn native_order_duplicates_and_repeated_anchors_are_deterministic() {
    let source = b"debugger;\ndebugger;\n";
    let a = project_eslint_findings(
        "app.js",
        "/fixture/app.js",
        source,
        &[finding(2, 1), finding(1, 1), finding(1, 1)],
    )
    .unwrap();
    let b = project_eslint_findings(
        "app.js",
        "/fixture/app.js",
        source,
        &[finding(1, 1), finding(2, 1)],
    )
    .unwrap();
    let mut wording = finding(1, 1);
    wording.message = "z alternative wording".into();
    let duplicate_wording = project_eslint_findings(
        "app.js",
        "/fixture/app.js",
        source,
        &[finding(1, 1), wording, finding(2, 1)],
    )
    .unwrap();
    assert_eq!(duplicate_wording, b);
    assert_eq!(a, b);
    assert_eq!(a.len(), 2);
    assert_ne!(a[0]["finding_id"], a[1]["finding_id"]);
}
#[test]
fn invalid_scope_or_one_bad_position_discards_all_task_inputs() {
    for path in ["../app.js", "/app.js", "src/../app.js", "src\\app.js", ""] {
        assert!(
            project_eslint_findings(path, "/fixture/app.js", b"debugger;", &[finding(1, 1)])
                .is_none()
        );
    }
    for bad in [finding(0, 1), finding(2, 1), finding(1, 11)] {
        assert!(
            project_eslint_findings(
                "app.js",
                "/fixture/app.js",
                b"debugger;",
                &[finding(1, 1), bad]
            )
            .is_none()
        );
    }
    let mut bad = finding(1, 1);
    bad.path = "/fixture/other.js".into();
    assert!(project_eslint_findings("app.js", "/fixture/app.js", b"debugger;", &[bad]).is_none());
}
#[test]
fn unicode_columns_use_utf16_and_native_line_terminators() {
    let source = "'😀';\r\ndebugger;\u{2028}debugger;";
    let records = project_eslint_findings(
        "app.js",
        "/fixture/app.js",
        source.as_bytes(),
        &[finding(1, 6), finding(2, 1), finding(3, 1)],
    )
    .unwrap();
    assert_eq!(records.len(), 3);
    assert!(
        project_eslint_findings(
            "app.js",
            "/fixture/app.js",
            source.as_bytes(),
            &[finding(1, 7)]
        )
        .is_none()
    );
}

#[test]
#[ignore = "requires existing explicit Node and ESLint 10.11.0; native UTF16 positions"]
fn native_utf16_and_line_terminator_positions_project_without_false_tasks() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-identity-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let source = "'😀'; debugger;\r\ndebugger;\u{2028}debugger;";
    std::fs::write(root.join("app.js"), source).unwrap();
    std::fs::write(
        root.join("eslint.config.cjs"),
        "module.exports=[{rules:{'no-debugger':'error'}}]",
    )
    .unwrap();
    let node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap();
    let entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap();
    let version = std::process::Command::new(&node)
        .args(["--", entry.to_str().unwrap(), "--version"])
        .output()
        .unwrap();
    assert_eq!(version.stdout, b"v10.11.0\n");
    let result = std::process::Command::new(node)
        .current_dir(&root)
        .args([
            "--",
            entry.to_str().unwrap(),
            "--no-config-lookup",
            "--config",
            root.join("eslint.config.cjs").to_str().unwrap(),
            "--format",
            "json",
            root.join("app.js").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let parsed = codeguard_adapters::parse_eslint_json(
        &result.stdout,
        "10.11.0",
        "10.11.0",
        &[root.join("app.js").to_str().unwrap().into()],
        Some(1),
        None,
    );
    assert!(parsed.local_coherent, "{:?}", parsed.reason);
    assert_eq!(parsed.findings.len(), 3);
    assert_eq!((parsed.findings[0].line, parsed.findings[0].column), (1, 7));
    assert_eq!((parsed.findings[1].line, parsed.findings[2].line), (2, 3));
    let records = project_eslint_findings(
        "app.js",
        root.join("app.js").to_str().unwrap(),
        source.as_bytes(),
        &parsed.findings,
    )
    .unwrap();
    assert_eq!(records.len(), 3);
    std::fs::remove_dir_all(root).unwrap();
}
