use codeguard_adapters::EslintCommand;

#[test]
#[ignore = "requires explicit Node; validates argv forwarding only, not native ESLint"]
fn real_node_forwards_literal_eslint_arguments_without_shell_expansion() {
    use codeguard_runtime::{ProcessSpec, Termination, run_process};
    use std::{
        collections::BTreeMap,
        fs,
        sync::atomic::AtomicBool,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-eslint-argv-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let entry = root.join("observe argv.cjs");
    let config = root.join("eslint.config.mjs");
    let source = root.join("source with spaces;$(touch injected).js");
    fs::write(
        &entry,
        "process.stdout.write(JSON.stringify(process.argv.slice(2)));\n",
    )
    .unwrap();
    fs::write(&config, "export default [];\n").unwrap();
    fs::write(&source, "const value = 1;\n").unwrap();
    let command = EslintCommand {
        node: std::env::var_os("CODEGUARD_NODE_BIN").unwrap().into(),
        entry,
        config: config.clone(),
        sources: vec![source.clone()],
        report: root.join("report.json"),
        max_warnings: Some(0),
    };
    let args = command.args().unwrap();
    let observed = run_process(
        &ProcessSpec {
            executable: command.node,
            args: args.clone(),
            cwd: root.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(10),
            output_limit_bytes: 64 * 1024,
        },
        &AtomicBool::new(false),
    );
    assert_eq!(observed.termination, Termination::Exited(0));
    let forwarded: Vec<String> = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(
        forwarded,
        args[2..]
            .iter()
            .map(|a| a.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(fs::read_to_string(config).unwrap(), "export default [];\n");
    assert_eq!(fs::read_to_string(source).unwrap(), "const value = 1;\n");
    assert!(!root.join("injected").exists());
    fs::remove_dir_all(root).unwrap();
}
