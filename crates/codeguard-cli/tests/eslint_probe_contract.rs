use codeguard_adapters::EslintCommand;
use codeguard_cli::{eslint_probe::run_eslint_probe, eslint_probe_request::EslintProbeRequest};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Fixture, EslintProbeRequest) {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-eslint-probe-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let node = root.join("node");
    let entry = root.join("eslint.cjs");
    let config = root.join("eslint.config.mjs");
    let source = root.join("app.js");
    for p in [&node, &entry, &config, &source] {
        fs::write(p, "fixture input").unwrap();
    }
    let expected_sha256 = BTreeMap::from_iter(
        [&node, &entry, &config, &source]
            .into_iter()
            .map(|p| (p.clone(), Sha256::digest(fs::read(p).unwrap()).into())),
    );
    let request = EslintProbeRequest {
        command: EslintCommand {
            node,
            entry,
            config,
            sources: vec![source],
            report: root.join("run.json"),
            max_warnings: None,
        },
        cwd: root.clone(),
        evidence_dir: root.clone(),
        run_id: "run".into(),
        expected_version: "10.0.0".into(),
        expected_sha256,
        deadline: Instant::now() + Duration::from_secs(10),
    };
    (Fixture(root), request)
}
#[test]
fn invalid_identity_scope_version_and_stale_slot_are_rejected_before_node() {
    for mode in 0..7 {
        let (_fixture, mut req) = fixture();
        let cancelled = AtomicBool::new(mode == 5);
        match mode {
            0 => {
                req.expected_sha256.remove(&req.command.config);
            }
            1 => {
                req.expected_sha256
                    .insert(req.command.node.clone(), [1; 32]);
            }
            2 => {
                req.expected_version = "9.0.0".into();
            }
            3 => {
                fs::write(&req.command.report, "old").unwrap();
            }
            4 => {
                req.run_id = "../escape".into();
            }
            6 => {
                req.deadline = Instant::now() - Duration::from_millis(1);
            }
            _ => {}
        }
        let observed = run_eslint_probe(&req, &cancelled);
        assert!(!observed.local_coherent);
        assert!(observed.parsed.is_none());
        assert!(observed.reason.is_some());
        assert!(!req.evidence_dir.join("run-version.log").exists());
    }
}
#[test]
#[ignore = "requires explicit Node; controlled JSON fixture, not native ESLint"]
fn real_node_controlled_reports_bind_version_inputs_and_native_exit_contract() {
    for mode in 0..8 {
        let (_fixture, mut req) = fixture();
        req.command.node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap().into();
        // 模式7：TS 方言源码加包根 tsconfig.json，真实 Node 写完报告后篡改 tsconfig。
        if mode == 7 {
            let source = req.cwd.join("app.ts");
            fs::write(&source, "const value: number = 1;\n").unwrap();
            fs::write(req.cwd.join("tsconfig.json"), "{\"compilerOptions\":{}}\n").unwrap();
            req.command.sources = vec![source];
        }
        let version = if mode == 4 {
            "v10.1.0\\n"
        } else {
            "v10.0.0\\n"
        };
        let severity = if mode == 2 { 2 } else { 1 };
        let count = if mode == 0 { 0 } else { 1 };
        let report = serde_json::json!([{"filePath":req.command.sources[0],"messages":if count==0 {serde_json::json!([])} else {serde_json::json!([{"ruleId":"fixture-rule","severity":severity,"message":"fixture diagnostic","line":1,"column":1}])},"suppressedMessages":[],"errorCount":if mode==2 {1}else{0},"warningCount":if count==1 && mode!=2 {1}else{0},"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
        let output = if mode == 3 {
            "bad report".to_owned()
        } else {
            report.to_string()
        };
        let mutation = if mode == 5 {
            format!(
                "fs.writeFileSync({},'changed');",
                serde_json::to_string(req.command.config.to_str().unwrap()).unwrap()
            )
        } else if mode == 7 {
            format!(
                "fs.writeFileSync({},JSON.stringify({{changed:true}}));",
                serde_json::to_string(req.cwd.join("tsconfig.json").to_str().unwrap()).unwrap()
            )
        } else {
            String::new()
        };
        let early_exit = if mode == 6 { "process.exit(2);" } else { "" };
        fs::write(&req.command.entry,format!("const fs=require('node:fs'); const args=process.argv.slice(2); if(args.includes('--version')){{process.stdout.write('{version}');process.exit(0);}} {early_exit} const target=args[args.indexOf('--output-file')+1]; fs.writeFileSync(target,{}); {mutation} process.exit({});",serde_json::to_string(&output).unwrap(),if mode==2 {1}else{0})).unwrap();
        let tsconfig = req.cwd.join("tsconfig.json");
        req.expected_sha256 = BTreeMap::from_iter(
            [
                &req.command.node,
                &req.command.entry,
                &req.command.config,
                &req.command.sources[0],
            ]
            .into_iter()
            .chain((mode == 7).then_some(&tsconfig))
            .map(|p| (p.clone(), Sha256::digest(fs::read(p).unwrap()).into())),
        );
        req.deadline = Instant::now() + Duration::from_secs(60);
        let observed = run_eslint_probe(&req, &AtomicBool::new(false));
        match mode {
            0..=2 => {
                assert!(observed.local_coherent, "{:?}", observed.reason);
                assert_eq!(observed.parsed.unwrap().findings.len(), count);
            }
            7 => {
                assert!(!observed.local_coherent, "{:?}", observed.reason);
                assert_eq!(observed.reason, Some("eslint_input_changed"));
                assert!(observed.parsed.is_some());
            }
            3 => {
                assert!(!observed.local_coherent);
                assert_eq!(observed.reason, Some("eslint_report_invalid"));
            }
            4 => {
                assert!(!observed.local_coherent);
                assert_eq!(observed.reason, Some("eslint_version_mismatch"));
                assert!(!req.command.report.exists());
            }
            5 => {
                assert!(!observed.local_coherent);
                assert_eq!(observed.reason, Some("eslint_input_changed"));
                assert!(observed.parsed.is_some());
            }
            6 => {
                assert!(!observed.local_coherent);
                assert_eq!(observed.reason, Some("eslint_report_read_failed"));
                assert!(observed.parsed.is_none());
            }
            _ => unreachable!(),
        }
    }
}

#[test]
#[cfg(unix)]
fn interrupted_scan_preserves_cancel_and_deadline_reason_with_or_without_report() {
    use std::os::unix::fs::PermissionsExt;
    for mode in 0..4 {
        let (_fixture, mut req) = fixture();
        let marker = req.cwd.join("scan-started");
        let clean = serde_json::json!([{"filePath":req.command.sources[0],"messages":[],"suppressedMessages":[],"errorCount":0,"warningCount":0,"fatalErrorCount":0,"fixableErrorCount":0,"fixableWarningCount":0}]);
        let output = if mode % 2 == 0 {
            format!("printf '%s' '{}' > \"$report\"\n", clean)
        } else {
            String::new()
        };
        fs::write(&req.command.node, format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf 'v10.0.0\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; shift; done\n{output}: > '{}'\n/bin/sleep 8\nexit 0\n", marker.display())).unwrap();
        fs::set_permissions(&req.command.node, fs::Permissions::from_mode(0o700)).unwrap();
        req.expected_sha256.insert(
            req.command.node.clone(),
            Sha256::digest(fs::read(&req.command.node).unwrap()).into(),
        );
        let cancel = AtomicBool::new(false);
        req.deadline = Instant::now() + Duration::from_secs(if mode < 2 { 10 } else { 5 });
        let observed = std::thread::scope(|scope| {
            if mode < 2 {
                let cancel_ref = &cancel;
                let marker_ref = &marker;
                scope.spawn(move || {
                    let stop = Instant::now() + Duration::from_secs(8);
                    while !marker_ref.exists() && Instant::now() < stop {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    cancel_ref.store(true, Ordering::Relaxed);
                });
            }
            run_eslint_probe(&req, &cancel)
        });
        assert!(marker.exists(), "scan must actually start");
        assert_eq!(req.command.report.exists(), mode % 2 == 0);
        assert!(!observed.local_coherent);
        assert_eq!(
            observed.reason,
            Some(if mode < 2 {
                "request_cancelled"
            } else {
                "request_deadline_exceeded"
            })
        );
    }
}

#[test]
#[ignore = "requires explicit Node and existing native ESLint 10.11.0 entry"]
fn real_eslint_observes_native_rules_suppression_parser_and_repair() {
    let (_fixture, mut req) = fixture();
    req.command.node = std::env::var_os("CODEGUARD_NODE_BIN").unwrap().into();
    req.command.entry = std::env::var_os("CODEGUARD_ESLINT_ENTRY").unwrap().into();
    req.expected_version = "10.11.0".into();
    req.command.config = req.cwd.join("eslint.config.cjs");
    fs::write(&req.command.config, "module.exports=[{files:['**/*.js'],rules:{'no-unused-vars':'warn','no-debugger':'error'}}];").unwrap();
    let source = req.command.sources[0].clone();
    for (index, contents) in [
        "console.log('ok');",
        "const unused=1;",
        "debugger;",
        "const = ;",
        "/* eslint-disable no-debugger */\ndebugger;",
        "console.log('repaired');",
    ]
    .iter()
    .enumerate()
    {
        fs::write(&source, contents).unwrap();
        req.run_id = format!("native-{index}");
        req.command.report = req.cwd.join(format!("{}.json", req.run_id));
        req.expected_sha256 = BTreeMap::from_iter(
            [
                &req.command.node,
                &req.command.entry,
                &req.command.config,
                &source,
            ]
            .into_iter()
            .map(|path| (path.clone(), Sha256::digest(fs::read(path).unwrap()).into())),
        );
        req.deadline = Instant::now() + Duration::from_secs(60);
        let result = run_eslint_probe(&req, &AtomicBool::new(false));
        let parsed = result.parsed.expect("native report must parse");
        match index {
            0 | 5 => {
                assert!(result.local_coherent, "{:?}", result.reason);
                assert!(parsed.findings.is_empty());
            }
            1 | 2 => {
                assert!(result.local_coherent, "{:?}", result.reason);
                assert_eq!(parsed.findings.len(), 1);
                assert_eq!(
                    parsed.findings[0].rule_id,
                    if index == 1 {
                        "no-unused-vars"
                    } else {
                        "no-debugger"
                    }
                );
            }
            3 => {
                assert!(!result.local_coherent);
                assert_eq!(
                    result.reason,
                    Some("eslint_parser_or_configuration_diagnostic")
                );
            }
            4 => {
                assert!(!result.local_coherent);
                assert_eq!(result.reason, Some("eslint_suppression_requires_review"));
                assert_eq!(parsed.suppressed_count, 1);
                assert!(parsed.findings.is_empty());
            }
            _ => unreachable!(),
        }
    }
}
