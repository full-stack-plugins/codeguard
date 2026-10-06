#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
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
        .join(format!("cg-syntax-lint-{name}-{}", std::process::id()));
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
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    if let Some(root) = std::env::var_os("CODEGUARD_SYNTAX_LINT_REPORT_DIR") {
        fs::create_dir_all(&root).unwrap();
        let path = PathBuf::from(root).join(format!(
            "{}-{}.json",
            value["selection"]["language"].as_str().unwrap_or("other"),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    value
}

#[test]
fn unknown_duplicate_or_foreign_tool_parameters_fail_before_side_effects() {
    let f = fixture("args");
    let source = f.0.join("a.dart");
    fs::write(&source, "void main() {}").unwrap();
    for tail in [
        vec!["not-a-language"],
        vec![
            "dart",
            source.to_str().unwrap(),
            "--timeout=1s",
            "--timeout",
            "2s",
        ],
        vec!["dart", source.to_str().unwrap(), "--cargo-tool", "/no/tool"],
        vec!["dart", source.to_str().unwrap(), "--format=garbage"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("lint")
            .args(tail)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
    }
    assert!(!f.0.join(".codeguard").exists());
}
#[test]
fn default_build_exposes_an_honest_native_adapter_gap() {
    let f = fixture("default");
    let source = f.0.join("a.dart");
    fs::write(&source, "void main() {}").unwrap();
    let report = run(&["lint", "dart", source.to_str().unwrap(), "--format=json"]);
    assert_eq!(report["report_type"], "syntax_lint_feedback");
    assert_eq!(report["native"]["reason"], "native_adapter_not_integrated");
    assert_eq!(report["native"]["configuration_status"], "unknown");
    assert_eq!(report["setup"]["requirement"], "required");
    assert_eq!(report["findings"], serde_json::json!([]));
    #[cfg(not(feature = "wasm-precheck"))]
    assert_eq!(report["reason"], "wasm_not_enabled");
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn every_remaining_grammar_has_a_registered_single_file_lint_route() {
    let f = fixture("coverage");
    for (language, name, text) in [
        (
            "arkts",
            "a.ets",
            "@Component struct C { build() { Text('hi') } }",
        ),
        ("c", "a.c", "int main(void) {return 0;}"),
        ("cpp", "a.cpp", "int main() {return 0;}"),
        ("csharp", "a.cs", "class C {}"),
        ("dart", "a.dart", "void main() {}"),
        ("cfml", "a.cfm", "<cfquery name=\"q\">SELECT 1</cfquery>"),
        ("cfml", "a.cfs", "component { function f() { return 1; } }"),
        (
            "cobol",
            "a.cbl",
            "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n",
        ),
        ("lua", "a.lua", "local x = 1"),
        ("luau", "a.luau", "local x: number = 1"),
        ("nix", "a.nix", "let x = 1; in x"),
        ("objc", "a.m", "@interface Foo : NSObject\n@end\n"),
        ("pascal", "a.pas", "program Hello;\nbegin\nend.\n"),
        ("php", "a.php", "<?php function f() {return 1;}"),
        ("r", "a.r", "x <- 1"),
        ("scala", "a.scala", "object Main {}"),
        (
            "solidity",
            "a.sol",
            "pragma solidity ^0.8.20; contract Vault {}",
        ),
        (
            "terraform",
            "a.tf",
            "resource \"x\" \"y\" { foo = \"bar\" }",
        ),
        ("vbnet", "a.vb", "Public Class C\nEnd Class\n"),
    ] {
        let source = f.0.join(name);
        fs::write(&source, text).unwrap();
        let report = run(&[
            "lint",
            language,
            source.to_str().unwrap(),
            "--format",
            "json",
            "--timeout=120s",
        ]);
        assert_eq!(report["selection"]["language"], language);
        let rows = report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap();
        assert!(!rows.is_empty(), "{name}: {report}");
        assert!(
            rows.iter()
                .all(|r| r["status"] == "candidate_observed" && r["grammar_qualified"] == false),
            "{name}: {report}"
        );
        assert_eq!(report["setup"]["requirement"], "required");
    }
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn standalone_task_reuses_project_identity_and_cannot_close_on_clean_candidate() {
    let f = fixture("task");
    let source = f.0.join("a.dart");
    let init = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["init", f.0.to_str().unwrap(), "--apply", "--format=json"])
        .output()
        .unwrap();
    assert_eq!(init.status.code(), Some(3));
    fs::write(&source, "void main( {\n").unwrap();
    let args = [
        "lint",
        "dart",
        source.to_str().unwrap(),
        "--workspace",
        f.0.to_str().unwrap(),
        "--format=json",
    ];
    let first = run(&args);
    let id = first["setup"]["task_id"].as_str().unwrap();
    let repeat = run(&args);
    assert_eq!(repeat["setup"]["task_id"], id);
    assert_eq!(repeat["syntax_tasks"]["new_blockers"], 0);
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "dart", f.0.to_str().unwrap(), "--format=json"])
        .env("PATH", "/no/tools")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let project: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        project["syntax_tasks"]["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["task_id"] == id),
        "{project}"
    );
    fs::write(&source, "void main() {}\n").unwrap();
    let clean = run(&args);
    assert_eq!(clean["setup"]["task_id"], id);
    assert_eq!(clean["setup"]["requirement"], "required");
    let projection = f.0.join(".codeguard/tasks").join(format!("{id}.md"));
    fs::remove_file(projection).unwrap();
    let damaged = run(&args);
    assert_eq!(damaged["syntax_tasks"]["status"], "incomplete");
    assert_eq!(damaged["setup"]["task_id"], Value::Null);
}
#[cfg(feature = "wasm-precheck")]
#[test]
fn mismatched_language_and_ambiguous_sources_never_use_another_grammar() {
    let f = fixture("scope");
    let source = f.0.join("a.php");
    fs::write(&source, "<?php echo 1;").unwrap();
    let report = run(&["lint", "dart", source.to_str().unwrap(), "--format=json"]);
    assert_eq!(
        report["reason"],
        "selected_language_or_grammar_scope_unresolved"
    );
    assert_eq!(report["syntax_candidates"], Value::Null);
    let header = f.0.join("a.h");
    fs::write(&header, "int x;").unwrap();
    let report = run(&["lint", "c", header.to_str().unwrap(), "--format=json"]);
    assert_eq!(report["syntax_candidates"], Value::Null);
    let child = f.0.join("child");
    fs::create_dir(&child).unwrap();
    let report = run(&[
        "lint",
        "php",
        source.to_str().unwrap(),
        "--workspace",
        child.to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(report["reason"], "source_scope_unavailable");
    assert!(!child.join(".codeguard").exists());
    let linked = f.0.join("linked.php");
    std::os::unix::fs::symlink(&source, &linked).unwrap();
    let report = run(&["lint", "php", linked.to_str().unwrap(), "--format=json"]);
    assert_eq!(report["reason"], "source_path_symlink_disallowed");
    assert_eq!(report["syntax_candidates"], Value::Null);
}
