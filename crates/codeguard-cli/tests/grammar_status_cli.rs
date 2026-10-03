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
    assert_eq!(report["schema_version"], "1.1.0");
    assert_eq!(report["codegraph_grammar_count"], 32);
    assert_eq!(report["codegraph_vendored_count"], 30);
    assert_eq!(report["candidate_count"], 32);
    assert_eq!(report["released_count"], 0);
    assert_eq!(report["authority"], "source_inventory_only");
    assert_eq!(report["parser_capability"], "unverified");
    assert_eq!(report["gate_effect"], "none");
    assert_eq!(report["execution"], "not_run");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["next_action"].as_str().unwrap().contains("原生优先"));
    assert!(
        !report["next_action"]
            .as_str()
            .unwrap()
            .contains("补齐缺口资产")
    );
    let assets = report["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 32);
    for asset in assets {
        assert_eq!(asset["released"], false);
        assert!(!asset["gap"].as_str().unwrap().is_empty());
        assert!(!asset["known_limitations"].as_array().unwrap().is_empty());
    }
    for language in [
        "arkts",
        "c",
        "cfml",
        "cfquery",
        "cfscript",
        "cobol",
        "cpp",
        "csharp",
        "dart",
        "erlang",
        "go",
        "java",
        "javascript",
        "kotlin",
        "lua",
        "luau",
        "nix",
        "objc",
        "pascal",
        "php",
        "python",
        "r",
        "ruby",
        "rust",
        "scala",
        "solidity",
        "swift",
        "terraform",
        "typescript",
        "tsx",
        "vbnet",
        "zig",
    ] {
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
    assert_eq!(cobol["gap"], "language_qualification_and_release_pending");
    assert!(cobol["source_bytes"].as_u64().unwrap() > 8 * 1024 * 1024);
    let dart = assets.iter().find(|row| row["language"] == "dart").unwrap();
    assert_eq!(dart["integration_status"], "candidate_unvalidated");
    assert_eq!(dart["gap"], "language_qualification_and_release_pending");
    assert_eq!(
        dart["source_sha256"],
        "7f5364e4256cf7e55efd01dd52421ef2663caa8061b82659b7e4bf61064545ec"
    );
    assert_eq!(
        dart["candidate_input_sha256"],
        "bbb37cc6aebca30188fab912bb2ae7201604d2b001a64ea6ef2c397674f81f5c"
    );
    assert_eq!(
        dart["candidate_wasm_sha256"],
        "7dad281b3b24924d619cb7059a42b409e7690ebeb8ebbc82882e68167656e012"
    );
    let zig = assets.iter().find(|row| row["language"] == "zig").unwrap();
    assert_eq!(zig["gap"], "language_qualification_and_release_pending");
    let vbnet = assets
        .iter()
        .find(|row| row["language"] == "vbnet")
        .unwrap();
    assert!(
        vbnet["known_limitations"][0]
            .as_str()
            .unwrap()
            .contains("known grammar false positive")
    );
    for (language, evidence) in [
        ("c", "C11 Apple clang 21 differential"),
        ("go", "Go 1.23.4 gofmt differential"),
        ("java", "Java 17 syntax corpus of 8 valid and 5 invalid"),
        ("javascript", "Node 24.18.0 module syntax differential"),
        ("rust", "Rust 2021 rustfmt 1.9.0 differential"),
        ("zig", "Zig 0.16.0 ast-check differential"),
        ("cfquery", "does not validate full SQL semantics"),
    ] {
        let asset = assets
            .iter()
            .find(|row| row["language"] == language)
            .unwrap();
        assert!(
            asset["known_limitations"][0]
                .as_str()
                .unwrap()
                .contains(evidence),
            "{language}: {asset}"
        );
        assert_eq!(asset["released"], false);
    }
    for language in ["objc", "solidity"] {
        let asset = assets
            .iter()
            .find(|row| row["language"] == language)
            .unwrap();
        assert_eq!(asset["gap"], "language_qualification_and_release_pending");
        assert_eq!(asset["source_sha256"].as_str().unwrap().len(), 64);
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
