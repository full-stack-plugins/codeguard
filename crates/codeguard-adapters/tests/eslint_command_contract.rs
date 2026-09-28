use codeguard_adapters::EslintCommand;
use std::path::PathBuf;
fn command() -> EslintCommand {
    EslintCommand {
        node: "/tools/node".into(),
        entry: "/tools/eslint/bin/eslint.js".into(),
        config: "/project/eslint.config.mjs".into(),
        sources: vec![
            "/project/source with spaces/app.ts".into(),
            "/project/a;$(echo injected).js".into(),
        ],
        report: "/private/run/report.json".into(),
        max_warnings: None,
    }
}
#[test]
fn node_entry_original_config_and_all_sources_are_literal_without_fix_or_install() {
    let actual: Vec<_> = command()
        .args()
        .unwrap()
        .into_iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        actual,
        [
            "--",
            "/tools/eslint/bin/eslint.js",
            "--no-config-lookup",
            "--config",
            "/project/eslint.config.mjs",
            "--format",
            "json",
            "--output-file",
            "/private/run/report.json",
            "--",
            "/project/source with spaces/app.ts",
            "/project/a;$(echo injected).js"
        ]
    );
    assert!(!actual.iter().any(|v| matches!(
        v.as_str(),
        "--fix" | "--quiet" | "--cache" | "--no-ignore" | "npx"
    )));
}
#[test]
fn warning_threshold_is_explicit_and_does_not_rewrite_rules_or_scope() {
    let mut input = command();
    input.max_warnings = Some(0);
    let args = input.args().unwrap();
    let strings: Vec<_> = args.iter().map(|v| v.to_str().unwrap()).collect();
    assert!(strings.windows(2).any(|w| w == ["--max-warnings", "0"]));
    assert_eq!(
        &strings[strings.len() - 2..],
        [
            "/project/source with spaces/app.ts",
            "/project/a;$(echo injected).js"
        ]
    );
}
#[test]
fn unsafe_empty_duplicate_or_colliding_inputs_are_rejected() {
    for path in [
        "relative.js",
        "/project/../app.js",
        "/project/./app.js",
        "/project/app\n.js",
        "/project/app\0.js",
    ] {
        let mut input = command();
        input.entry = PathBuf::from(path);
        assert!(input.args().is_err(), "{path:?}");
    }
    let mut input = command();
    input.sources.clear();
    assert!(input.args().is_err());
    for mode in 0..6 {
        let mut input = command();
        match mode {
            0 => input.report = input.config.clone(),
            1 => input.report = input.node.clone(),
            2 => input.report = input.entry.clone(),
            3 => input.report = input.sources[0].clone(),
            4 => input.sources.push(input.sources[0].clone()),
            _ => input.config = input.entry.clone(),
        };
        assert!(input.args().is_err());
    }
}
#[test]
fn planning_budget_is_bounded_before_any_process_can_start() {
    let mut input = command();
    input.sources = (0..10_001)
        .map(|i| PathBuf::from(format!("/project/{i}.js")))
        .collect();
    assert!(input.args().is_err());
    let mut input = command();
    input.sources = (0..100)
        .map(|i| PathBuf::from(format!("/project/{}/{}.js", "a".repeat(2048), i)))
        .collect();
    assert!(input.args().is_err());
}
