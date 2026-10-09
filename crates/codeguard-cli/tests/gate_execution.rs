//! CLI gate evidence boundaries. Clean source alone cannot grant project allow;
//! malformed source alone cannot prove complete coverage and grant project deny.
use serde_json::Value;
use std::{fs, process::Command};

fn unqualified_java(name: &str, source: &str) {
    let root = std::env::temp_dir().join(format!("codeguard-gate-{name}-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Sample.java"), source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["exit_code"], 3);
    assert_eq!(report["command_status"], "incomplete");
    assert_eq!(report["delivery_decision"], "incomplete");
    assert_eq!(report["authority"], "local_unverified");
    assert_eq!(report["obligation_status"], "unresolved");
    let unresolved = report["unresolved_conditions"]
        .as_array()
        .expect("missing gate diagnostics");
    assert!(
        unresolved.iter().any(|x| x == "trusted_policy_unavailable"),
        "{report}"
    );
    assert!(
        unresolved
            .iter()
            .any(|x| x == "required_obligations_unresolved"),
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_java_without_complete_evidence_is_incomplete() {
    unqualified_java("invalid", "class Sample { int f( { return 1; } }\n");
}

#[test]
fn clean_java_without_complete_evidence_is_incomplete() {
    unqualified_java("clean", "class Sample { int x = 1; }\n");
}
