use codeguard_adapters::parse_eslint_effective_rule;
#[test]
fn native_rule_levels_are_exact_and_missing_rule_is_disabled() {
    for (value, expected) in [
        ("0", 0),
        ("1", 1),
        ("2", 2),
        ("\"off\"", 0),
        ("[2,{\"option\":true}]", 2),
    ] {
        let raw =
            format!("{{\"rules\":{{\"no-debugger\":{value}}},\"settings\":{{\"private\":true}}}}");
        assert_eq!(
            parse_eslint_effective_rule(raw.as_bytes(), "no-debugger"),
            Ok(Some(expected))
        );
    }
    assert_eq!(
        parse_eslint_effective_rule(br#"{"rules":{}}"#, "no-debugger"),
        Ok(None)
    );
}
#[test]
fn malformed_or_ambiguous_native_settings_are_not_rule_coverage() {
    assert_eq!(
        parse_eslint_effective_rule(b"undefined\n", "x"),
        Err("eslint_effective_target_not_selected")
    );
    for raw in [
        r#"{"rules":{"x":3}}"#,
        r#"{"rules":{"x":[]}}"#,
        r#"{"rules":{"x":2,"x":0}}"#,
        r#"{"rules":{},"rules":{}}"#,
        r#"{"rules":null}"#,
        r#"{"settings":{}}"#,
        "undefined\n",
        "[]",
    ] {
        assert!(
            parse_eslint_effective_rule(raw.as_bytes(), "x").is_err(),
            "{raw}"
        );
    }
}

#[test]
#[cfg(unix)]
fn print_config_plan_preserves_original_paths_and_rejects_multi_file_scope() {
    let mut command = codeguard_adapters::EslintCommand {
        node: "/tmp/node".into(),
        entry: "/tmp/eslint.cjs".into(),
        config: "/tmp/original.cjs".into(),
        sources: vec!["/tmp/source.js".into()],
        report: "/tmp/run.json".into(),
        max_warnings: None,
    };
    let args: Vec<_> = command
        .effective_config_args()
        .unwrap()
        .into_iter()
        .map(|a| a.into_string().unwrap())
        .collect();
    assert_eq!(
        args,
        [
            "--",
            "/tmp/eslint.cjs",
            "--no-config-lookup",
            "--config",
            "/tmp/original.cjs",
            "--print-config",
            "/tmp/source.js"
        ]
    );
    command.sources.push("/tmp/other.js".into());
    assert!(command.effective_config_args().is_err());
    command.sources = vec![command.config.clone()];
    assert!(
        command.args().is_ok(),
        "original configuration is also source code"
    );
    command.sources.push(command.config.clone());
    assert!(command.args().is_err(), "duplicate sources remain invalid");
    command.sources = vec![command.report.clone()];
    assert!(command.args().is_err(), "report cannot alias source");
}
