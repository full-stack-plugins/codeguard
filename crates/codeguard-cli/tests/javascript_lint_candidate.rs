#![cfg(all(unix, feature = "wasm-precheck"))]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
fn capture(report: &Value) {
    if let Some(directory) = std::env::var_os("CODEGUARD_JS_LINT_REPORT_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!(
                "{}.json",
                REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            )),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }
}
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(name: &str) -> Fixture {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-js-lint-{name}-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    Fixture(root)
}
fn run(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report = serde_json::from_slice(&output.stdout).unwrap();
    capture(&report);
    report
}
#[test]
fn standalone_javascript_duplicates_reuse_project_confirmation_identity() {
    let f = fixture("workbench");
    run(&["init", f.0.to_str().unwrap(), "--apply", "--format=json"]);
    for suffix in ["js", "mjs", "cjs", "jsx"] {
        let source = f.0.join(format!("input.{suffix}"));
        fs::write(&source, "const x=1; const x=2;\n").unwrap();
        let args = [
            "lint",
            "typescript",
            source.to_str().unwrap(),
            "--workspace",
            f.0.to_str().unwrap(),
            "--format=json",
        ];
        let first = run(&args);
        assert_eq!(first["schema_version"], "0.6.0", "{first}");
        let row = &first["syntax_candidates"]["observations"][0];
        assert_eq!(row["language"], "javascript");
        assert_eq!(row["recovery_count"], 0);
        assert_eq!(row["structural_observation_count"], 1);
        assert_eq!(first["setup"]["requirement"], "required");
        let id = first["setup"]["task_id"].as_str().unwrap();
        let repeat = run(&args);
        assert_eq!(repeat["setup"]["task_id"], id);
        assert_eq!(repeat["syntax_tasks"]["new_blockers"], 0);
        let project = run(&["check", "all", f.0.to_str().unwrap(), "--format=json"]);
        assert!(
            project["syntax_tasks"]["tasks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["task_id"] == id),
            "{project}"
        );
        fs::write(&source, "let x; {let x;}\n").unwrap();
        let clean = run(&args);
        assert_eq!(clean["setup"]["requirement"], "required", "{clean}");
        assert_eq!(
            clean["setup"]["reason"],
            "native_confirmation_still_pending"
        );
        assert_eq!(clean["setup"]["task_id"], id);
        assert_eq!(clean["syntax_tasks"]["tasks"], json!([]));
        let fact: Value = serde_json::from_slice(
            &fs::read(
                f.0.join(".codeguard/findings")
                    .join(id)
                    .join("finding.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open", "{fact}");
    }
}
#[test]
fn unbound_and_commonjs_return_feedback_recommends_native_without_inventing_error() {
    let f = fixture("unbound");
    for (suffix, text) in [
        ("cjs", "return 1;"),
        ("js", "var x; var x;"),
        ("mjs", "const x='const x=2';"),
    ] {
        let source = f.0.join(format!("input.{suffix}"));
        fs::write(&source, text).unwrap();
        let report = run(&[
            "lint",
            "typescript",
            source.to_str().unwrap(),
            "--format=json",
        ]);
        assert_eq!(report["schema_version"], "0.6.0", "{report}");
        assert_eq!(report["setup"]["requirement"], "recommended");
        assert_eq!(report["setup"]["task_id"], Value::Null);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert!(!f.0.join(".codeguard").exists());
        assert_eq!(fs::read_to_string(source).unwrap(), text);
    }
    let source = f.0.join("input.mjs");
    let args = [
        "lint",
        "typescript",
        source.to_str().unwrap(),
        "--workspace",
        f.0.to_str().unwrap(),
        "--format=json",
    ];
    let uninitialized = run(&args);
    assert_eq!(uninitialized["setup"]["requirement"], "required");
    assert_eq!(
        uninitialized["syntax_tasks"]["failures"][0]["reason"],
        "workspace_not_initialized"
    );
    run(&["init", f.0.to_str().unwrap(), "--apply", "--format=json"]);
    let no_history = run(&args);
    assert_eq!(
        no_history["setup"]["requirement"], "recommended",
        "{no_history}"
    );
    assert_eq!(no_history["syntax_tasks"]["failures"], json!([]));
    fs::write(f.0.join(".codeguard/workspace.json"), "{").unwrap();
    let invalid = run(&args);
    assert_eq!(invalid["setup"]["requirement"], "required");
    assert_eq!(
        invalid["syntax_tasks"]["failures"][0]["reason"],
        "workspace_invalid"
    );
}

#[test]
fn relative_source_and_native_confirmation_required_for_parser_recovery() {
    let f = fixture("relative");
    fs::write(f.0.join("input.js"), "const x = ;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .current_dir(&f.0)
        .args(["lint", "typescript", "input.js", "--format=json"])
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    capture(&report);
    assert!(
        report["syntax_candidates"]["observations"][0]["recovery_count"]
            .as_u64()
            .unwrap()
            > 0,
        "{report}"
    );
    assert_eq!(report["setup"]["requirement"], "required");
    assert_eq!(report["syntax_tasks"], Value::Null);
}

#[test]
fn missing_workspace_and_failed_persistence_keep_candidate_without_fabricated_task() {
    use std::os::unix::fs::symlink;
    let f = fixture("persist");
    let source = f.0.join("input.js");
    fs::write(&source, "const x=1; const x=2;\n").unwrap();
    let args = [
        "lint",
        "typescript",
        source.to_str().unwrap(),
        "--workspace",
        f.0.to_str().unwrap(),
        "--format=json",
    ];
    let absent = run(&args);
    assert_eq!(
        absent["syntax_tasks"]["failures"][0]["reason"],
        "workspace_not_initialized"
    );
    assert_eq!(absent["setup"]["task_id"], Value::Null);
    assert!(
        absent["next_action"]
            .as_str()
            .unwrap()
            .contains("codeguard init")
    );
    run(&["init", f.0.to_str().unwrap(), "--apply", "--format=json"]);
    let reports = f.0.join(".codeguard/reports");
    if reports.exists() {
        fs::remove_dir(&reports).unwrap();
    }
    let outside = f.0.join("foreign_reports");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("marker"), "preserve").unwrap();
    symlink(&outside, &reports).unwrap();
    let failed = run(&args);
    assert_eq!(failed["setup"]["task_id"], Value::Null);
    assert_eq!(failed["workbench_status"], "incomplete");
    assert_eq!(
        failed["syntax_candidates"]["observations"][0]["structural_observation_count"],
        1
    );
    assert_eq!(fs::read_dir(outside).unwrap().count(), 1);
}
#[test]
fn outside_workspace_cannot_create_confirmation_and_human_feedback_redacts_names() {
    let f = fixture("boundary");
    let child = f.0.join("child");
    fs::create_dir(&child).unwrap();
    let source = f.0.join("input.js");
    fs::write(
        &source,
        "const PRIVATE_IDENTIFIER_TOKEN=1; const PRIVATE_IDENTIFIER_TOKEN=2;\n",
    )
    .unwrap();
    let outside = run(&[
        "lint",
        "typescript",
        source.to_str().unwrap(),
        "--workspace",
        child.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(outside["syntax_candidates"]["status"], "not_run");
    assert_eq!(outside["setup"]["requirement"], "required");
    assert!(!child.join(".codeguard").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["lint", "typescript"])
        .arg(&source)
        .arg("--format=human")
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("codeguard.javascript.duplicate_direct_lexical_binding"),
        "{text}"
    );
    assert!(text.contains("UTF-8"));
    assert!(!text.contains("PRIVATE_IDENTIFIER_TOKEN"));
}

#[test]
fn damaged_history_never_downgrades_pending_native_confirmation() {
    use std::os::unix::fs::symlink;
    let f = fixture("history");
    run(&["init", f.0.to_str().unwrap(), "--apply", "--format=json"]);
    let source = f.0.join("input.js");
    fs::write(&source, "const x=1; const x=2;\n").unwrap();
    let args = [
        "lint",
        "typescript",
        source.to_str().unwrap(),
        "--workspace",
        f.0.to_str().unwrap(),
        "--format=json",
    ];
    let first = run(&args);
    let id = first["setup"]["task_id"].as_str().unwrap();
    let fact =
        f.0.join(".codeguard/findings")
            .join(id)
            .join("finding.json");
    let projection = f.0.join(".codeguard/tasks").join(format!("{id}.md"));
    let original = fs::read(&fact).unwrap();
    let markdown = fs::read(&projection).unwrap();
    fs::write(&source, "const x=1;\n").unwrap();
    for mutation in [
        "bad_json",
        "missing_projection",
        "linked_projection",
        "forged_closed",
    ] {
        fs::write(&fact, &original).unwrap();
        if fs::symlink_metadata(&projection).is_ok() {
            fs::remove_file(&projection).unwrap();
        }
        fs::write(&projection, &markdown).unwrap();
        match mutation {
            "bad_json" => fs::write(&fact, "{").unwrap(),
            "missing_projection" => fs::remove_file(&projection).unwrap(),
            "linked_projection" => {
                let foreign = f.0.join("foreign.md");
                fs::write(&foreign, "DO_NOT_EXECUTE_HISTORY_TEXT").unwrap();
                fs::remove_file(&projection).unwrap();
                symlink(&foreign, &projection).unwrap();
            }
            "forged_closed" => {
                let mut value: Value = serde_json::from_slice(&original).unwrap();
                value["state"] = json!("closed");
                fs::write(&fact, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        let report = run(&args);
        assert_eq!(
            report["setup"]["requirement"], "required",
            "{mutation}: {report}"
        );
        assert_eq!(report["setup"]["task_id"], Value::Null);
        assert_eq!(report["workbench_status"], "incomplete");
        assert!(
            !report["syntax_tasks"]["failures"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(!report.to_string().contains("DO_NOT_EXECUTE_HISTORY_TEXT"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "const x=1;\n");
    }
}
