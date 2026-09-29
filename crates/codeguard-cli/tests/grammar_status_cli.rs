use serde_json::Value;
use std::process::Command;

#[test]
fn codegraph_coverage_is_explicit_and_never_claims_parser_or_gate_completion() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "status", "--format=json"])
        .output()
        .expect("grammar status");
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["report_type"], "grammar_coverage_inventory");
    assert_eq!(report["codegraph_grammar_count"], 32);
    assert_eq!(report["codegraph_vendored_count"], 30);
    assert_eq!(report["candidate_count"], 5);
    assert_eq!(report["released_count"], 0);
    assert_eq!(report["authority"], "source_inventory_only");
    assert_eq!(report["parser_capability"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    let assets = report["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 32);
    for asset in assets {
        assert_eq!(asset["released"], false);
        assert!(!asset["gap"].as_str().unwrap().is_empty());
    }
    for language in ["java", "python", "typescript", "tsx", "zig"] {
        let asset = assets
            .iter()
            .find(|row| row["language"] == language)
            .unwrap();
        assert_eq!(asset["integration_status"], "candidate_unvalidated");
        assert_eq!(asset["runtime_observation"], "rust_loader_smoke_passed");
    }
    let cobol = assets
        .iter()
        .find(|row| row["language"] == "cobol")
        .unwrap();
    assert_eq!(cobol["gap"], "current_loader_size_limit");
    assert!(cobol["source_bytes"].as_u64().unwrap() > 8 * 1024 * 1024);
    let zig = assets.iter().find(|row| row["language"] == "zig").unwrap();
    assert_eq!(zig["gap"], "language_qualification_and_release_pending");
    for language in ["objc", "solidity"] {
        let asset = assets
            .iter()
            .find(|row| row["language"] == language)
            .unwrap();
        assert_eq!(asset["gap"], "dependency_bytes_not_pinned");
        assert!(asset["source_sha256"].is_null());
    }
}

#[test]
fn grammar_status_rejects_unexpected_arguments_without_scanning() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "status", "--check"])
        .output()
        .expect("invalid grammar command");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
