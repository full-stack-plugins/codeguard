use serde_json::Value;
use std::{collections::BTreeSet, fs, process::Command};

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(args)
        .env("PATH", "")
        .output()
        .unwrap()
}

#[test]
fn json_help_covers_all_tracking_ids_and_keeps_quality_unevaluated() {
    let out = invoke(&["help", "--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["report_type"], "command_help");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["native_execution"], "not_run");
    let entries = report["commands"].as_array().unwrap();
    let ids = entries
        .iter()
        .filter_map(|r| r["tracking_id"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 36);
    for id in 1..=36 {
        assert!(ids.contains(format!("C{id:02}").as_str()));
    }
    let mcp = entries
        .iter()
        .find(|r| r["command"] == "mcp serve")
        .unwrap();
    assert_eq!(mcp["support"], "planned");
    assert_eq!(mcp["executable"], false);
    let probe = entries
        .iter()
        .find(|row| row["command"] == "grammar probe")
        .unwrap();
    assert_eq!(probe["executable"], cfg!(feature = "wasm-precheck"));
    assert_eq!(
        probe["support"],
        if cfg!(feature = "wasm-precheck") {
            "partial"
        } else {
            "unavailable_build"
        }
    );
    let lint = entries.iter().find(|r| r["command"] == "lint").unwrap();
    assert_eq!(
        lint["support"],
        if cfg!(unix) {
            "partial"
        } else {
            "unavailable_build"
        }
    );
    assert!(
        !lint["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "ruby")
    );
}
#[test]
fn command_query_and_help_aliases_are_read_only_and_consistent() {
    let root = std::env::temp_dir().join(format!("cg-help-readonly-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("AGENTS.md"), "human instructions").unwrap();
    for alias in ["help", "--help", "-h"] {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .current_dir(&root)
            .args([alias, "mcp", "serve", "--format=json"])
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["commands"].as_array().unwrap().len(), 1);
        assert_eq!(report["commands"][0]["executable"], false);
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "human instructions"
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn invalid_help_parameters_do_not_emit_a_success_packet() {
    for args in [
        vec!["help", "scan", "--format=json"],
        vec!["help", "--format", "sarif"],
        vec!["help", "--format=json", "--format=human"],
        vec!["help", "check", ".", "--format=json"],
    ] {
        let out = invoke(&args);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn human_help_shows_current_boundaries_and_no_stale_protocol_claim() {
    let out = invoke(&["--help"]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("C01"));
    assert!(text.contains("C36"));
    assert!(text.contains("not_evaluated"));
    assert!(!text.contains("check_feedback 0.34.0"));
    assert!(text.contains("help COMMAND"));
}

#[test]
fn exact_command_help_suffix_and_version_query_are_static() {
    for args in [
        vec!["check", "--help"],
        vec!["task", "verify", "-h"],
        vec!["mcp", "serve", "--help"],
        vec!["help", "--version", "--format=json"],
    ] {
        let out = invoke(&args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.stdout.is_empty());
    }
    let out = invoke(&["help", "task", "verify", "--format=json"]);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let examples = report["commands"][0]["examples"].as_array().unwrap();
    assert!(
        examples
            .iter()
            .any(|example| example.as_str().unwrap().contains("--go-tool"))
    );
    assert!(
        examples
            .iter()
            .any(|example| example.as_str().unwrap().contains("--erl-tool"))
    );
}
