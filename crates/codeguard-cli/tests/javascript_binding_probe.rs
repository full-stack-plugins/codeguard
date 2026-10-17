#![cfg(feature = "wasm-precheck")]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Source(PathBuf);
impl Drop for Source {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn probe(source: &str) -> Value {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-js-binding-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let fixture = Source(root);
    let path = fixture.0.join("input.js");
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "javascript"])
        .arg(&path)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), source);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    if let Some(directory) = std::env::var_os("CODEGUARD_JS_BINDING_REPORT_DIR") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join(format!(
                "{}.json",
                fixture.0.file_name().unwrap().to_str().unwrap()
            )),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    report
}
#[test]
fn public_probe_reports_duplicate_binding_separately_from_parser_recovery() {
    let report = probe("const x=1; const x=2;\n");
    assert_eq!(report["schema_version"], "0.6.0", "{report}");
    assert_eq!(report["recoveries"].as_array().unwrap().len(), 0);
    let structures = report["structural_observations"].as_array().unwrap();
    assert_eq!(structures.len(), 1);
    assert_eq!(
        structures[0]["rule_id"],
        "codeguard.javascript.duplicate_direct_lexical_binding"
    );
    assert_eq!(report["grammar_qualified"], false);
    assert_eq!(
        report["structural_rule_scope"],
        "direct_simple_lexical_names_only"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["next_action"],
        "confirm_candidate_structure_with_applicable_native_tool"
    );
}
#[test]
fn public_probe_does_not_guess_commonjs_return_or_nested_scopes() {
    for source in [
        "return 1;",
        "let x; {let x;}",
        "var x; var x;",
        "const x='const x=2';",
    ] {
        let report = probe(source);
        assert_eq!(report["schema_version"], "0.1.0", "{report}");
        assert!(report.get("structural_observations").is_none());
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
}

#[test]
fn public_probe_preserves_unicode_and_record_budget_incomplete() {
    let report = probe("let 名字;\r\nconst 名字=1;\r\n");
    let fact = &report["structural_observations"][0];
    assert_eq!(fact["start_row"], 1);
    assert_eq!(fact["start_column_byte"], 6);
    assert_eq!(
        fact["rule_sha256"],
        codeguard_adapters::javascript_binding_rule_sha256()
    );
    let report = probe(&"let x;\n".repeat(130));
    assert_eq!(
        report["structural_observations"].as_array().unwrap().len(),
        128,
        "{report}"
    );
    assert_eq!(report["precheck"]["truncated_files"], 1);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[cfg(unix)]
#[test]
fn parent_reader_rejects_rule_position_version_and_context_forgery() {
    use codeguard_cli::syntax_worker_runner::{
        run_syntax_worker_binding_candidate, run_syntax_worker_candidate,
    };
    use std::{
        io::Write,
        os::unix::fs::PermissionsExt,
        process::Stdio,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let source = b"const x=1; const x=2;\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["__syntax-worker", "javascript", "--direct-bindings"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(source).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let original: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(original["schema_version"], "1.5.0");
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-js-binding-reader-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let fixture = Source(root);
    let tool = fixture.0.join("worker");
    let write_report = |report: &Value| {
        fs::write(
            &tool,
            format!(
                "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat <<'CG_REPORT'\n{report}\nCG_REPORT\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    };
    let read = || {
        run_syntax_worker_binding_candidate(
            &tool,
            "javascript",
            "input.js",
            source,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        )
    };
    write_report(&original);
    assert_eq!(read().unwrap().structural_observations.len(), 1);
    assert!(
        run_syntax_worker_candidate(
            &tool,
            "javascript",
            "input.js",
            source,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    for (pointer, value) in [
        ("/schema_version", serde_json::json!("1.0.0")),
        ("/schema_version", serde_json::json!("2.0.0")),
        (
            "/structural_observations/0/rule_sha256",
            serde_json::json!("0".repeat(64)),
        ),
        (
            "/structural_observations/0/rule_id",
            serde_json::json!("unknown.rule"),
        ),
        (
            "/structural_observations/0/start_column_byte",
            serde_json::json!(9999),
        ),
        ("/structural_observations/0/end_byte", serde_json::json!(0)),
        ("/grammar_sha256", serde_json::json!("0".repeat(64))),
        ("/source_sha256", serde_json::json!("0".repeat(64))),
    ] {
        let mut forged = original.clone();
        *forged.pointer_mut(pointer).unwrap() = value;
        write_report(&forged);
        assert!(read().is_err(), "{pointer}");
    }
}
