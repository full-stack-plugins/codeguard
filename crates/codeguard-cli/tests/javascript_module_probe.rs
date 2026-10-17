#![cfg(all(feature = "wasm-precheck", unix))]
use serde_json::Value;
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn probe(source: &str, module: bool) -> Value {
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-js-module-{}-{}",
        std::process::id(),
        serial
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("input.js");
    fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
    command.args(["grammar", "probe", "javascript"]).arg(&path);
    if module {
        command.arg("--module");
    }
    command.arg("--format=json");
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODULE_REPORT_DIR") {
        let dir = std::path::PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("probe-{}-{}.json", u8::from(module), serial)),
            &output.stdout,
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
    report
}
#[test]
fn explicit_module_reports_outer_return_and_default_does_not_guess_mode() {
    for source in [
        "return 1;",
        "if(true){return 2;}",
        "while(false){return 3;}",
    ] {
        let report = probe(source, true);
        assert_eq!(report["schema_version"], "0.8.0");
        assert_eq!(report["javascript_mode"], "module");
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert!(report["recoveries"].as_array().unwrap().is_empty());
        assert_eq!(
            report["structural_observations"][0]["rule_id"],
            "codeguard.javascript.module_return_outside_function"
        );
        assert_eq!(
            report["structural_observations"][0]["rule_sha256"],
            codeguard_adapters::javascript_module_return_rule_sha256()
        );
        let default = probe(source, false);
        assert!(default.get("javascript_mode").is_none());
        assert!(default.get("structural_observations").is_none());
    }
}
#[test]
fn module_keeps_function_returns_legal_and_preserves_other_rule_evidence() {
    for source in [
        "function f(){return 1;}",
        "const f=function(){return 1;};",
        "function* f(){return 1;}",
        "const f=function*(){return 1;};",
        "const f=()=>{return 1;};",
        "class C{ f(){return 1;} }",
        "const o={f(){return 1;}};",
        "const s='return 1;';",
    ] {
        let report = probe(source, true);
        assert_eq!(report["schema_version"], "0.8.0");
        assert_eq!(report["javascript_mode"], "module");
        assert!(
            report["structural_observations"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{source}"
        );
        assert_eq!(report["native"]["status"], "not_run");
    }
    let report = probe("const x=1; const x=2; return 1;", true);
    let rows = report["structural_observations"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|row| row["rule_id"] == "codeguard.javascript.duplicate_direct_lexical_binding")
    );
    assert!(
        rows.iter()
            .any(|row| row["rule_id"] == "codeguard.javascript.module_return_outside_function")
    );
}
#[test]
fn module_argument_is_javascript_only_and_other_modes_are_rejected() {
    for args in [
        vec![
            "grammar",
            "probe",
            "python",
            "missing.py",
            "--module",
            "--format=json",
        ],
        vec![
            "grammar",
            "probe",
            "javascript",
            "missing.js",
            "--commonjs",
            "--format=json",
        ],
        vec!["__syntax-worker", "python", "--javascript-module"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn module_worker_context_rule_and_version_cannot_cross_legacy_boundaries() {
    use codeguard_cli::syntax_worker_runner::{
        run_syntax_worker_binding_candidate, run_syntax_worker_module_candidate,
    };
    use std::{
        io::Write,
        os::unix::fs::PermissionsExt,
        process::Stdio,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let source = b"return 1;";
    let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["__syntax-worker", "javascript", "--javascript-module"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    command.stdin.take().unwrap().write_all(source).unwrap();
    let output = command.wait_with_output().unwrap();
    assert!(output.status.success());
    let original: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(original["schema_version"], "1.7.0");
    assert_eq!(original["javascript_mode"], "module");
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-js-module-worker-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fake = root.join("worker");
    let payload = root.join("report.json");
    assert!(!payload.to_str().unwrap().contains('\''));
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat '{}'\n",
            payload.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&payload, &output.stdout).unwrap();
    let replay = || {
        run_syntax_worker_module_candidate(
            &fake,
            "javascript",
            "source.js",
            source,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false),
        )
    };
    assert_eq!(replay().unwrap().structural_observations.len(), 1);
    assert!(
        run_syntax_worker_binding_candidate(
            &fake,
            "javascript",
            "source.js",
            source,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    for mutation in [
        "old_version",
        "commonjs",
        "missing_mode",
        "null_mode",
        "wrong_rule_digest",
        "wrong_rule_kind",
        "unknown_field",
    ] {
        let mut value = original.clone();
        match mutation {
            "old_version" => value["schema_version"] = "1.5.0".into(),
            "commonjs" => value["javascript_mode"] = "commonjs".into(),
            "missing_mode" => {
                value.as_object_mut().unwrap().remove("javascript_mode");
            }
            "null_mode" => value["javascript_mode"] = Value::Null,
            "wrong_rule_digest" => {
                value["structural_observations"][0]["rule_sha256"] = "0".repeat(64).into()
            }
            "wrong_rule_kind" => {
                value["structural_observations"][0]["parent_syntax_kind"] = "program".into()
            }
            _ => value["approved"] = true.into(),
        }
        fs::write(&payload, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(replay().is_err(), "{mutation}");
    }
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODULE_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            std::path::PathBuf::from(dir).join("worker-original.json"),
            output.stdout,
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn module_and_binding_facts_share_one_output_budget() {
    let report = probe(&("let x;".repeat(129) + "return 1;"), true);
    assert_eq!(report["schema_version"], "0.8.0");
    assert_eq!(report["precheck"]["truncated_files"], 1);
    assert_eq!(
        report["structural_observations"].as_array().unwrap().len(),
        128
    );
    assert_eq!(report["status"], "incomplete");
}
#[test]
fn unreadable_module_source_retains_requested_context() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "grammar",
            "probe",
            "javascript",
            "/nonexistent/cg-module-source.js",
            "--module",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.8.0");
    assert_eq!(report["javascript_mode"], "module");
    assert!(report.get("structural_observations").is_none());
    assert!(report.get("reason").is_some());
    if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODULE_REPORT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            std::path::PathBuf::from(dir).join("module-unreadable.json"),
            output.stdout,
        )
        .unwrap();
    }
}
