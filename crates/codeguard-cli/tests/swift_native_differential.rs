#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("plain", "let value = 1\n", true),
        (
            "function",
            "func add(_ a: Int, _ b: Int) -> Int { a + b }\n",
            true,
        ),
        ("struct", "struct Point { let x: Int; let y: Int }\n", true),
        ("enum", "enum Status { case ready, failed(String) }\n", true),
        ("optional", "let name: String? = nil\n", true),
        ("closure", "let double = { (x: Int) in x * 2 }\n", true),
        ("interpolation", "let text = \"hello \\(1 + 2)\"\n", true),
        (
            "async",
            "func fetch() async throws -> Int { return 1 }\n",
            true,
        ),
        ("missing_brace", "func f() {\n", false),
        ("unclosed_string", "let s = \"oops\n", false),
        ("missing_expr", "let x =\n", false),
        ("bad_param", "func f(_ x: ) {}\n", false),
        ("bad_call", "print(1, 2\n", false),
    ]
}

fn candidate_report(root: &Path, name: &str, source: &str) -> serde_json::Value {
    let file = root.join(format!("{name}.swift"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "swift"])
        .arg(&file)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(candidate.status.code(), Some(3), "{name}: {candidate:?}");
    let report: serde_json::Value = serde_json::from_slice(&candidate.stdout).unwrap();
    assert_eq!(report["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["reason"],
        serde_json::Value::Null,
        "{name}: {report}"
    );
    report
}

#[test]
fn known_swift_missing_parameter_type_gap_cannot_claim_qualification() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-swift-gap-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let report = candidate_report(&root, "bad_param", "func f(_ x: ) {}\n");
    fs::remove_dir_all(root).unwrap();
    assert!(report["recoveries"].as_array().unwrap().is_empty());
    assert_eq!(report["status"], "incomplete");
}

#[test]
#[ignore = "requires explicit existing Apple Swift 6.4 via CODEGUARD_SWIFTC_BIN"]
fn swift_native_parse_exposes_one_known_candidate_false_negative() {
    let swiftc = std::env::var("CODEGUARD_SWIFTC_BIN")
        .expect("provide an existing Apple Swift compiler executable");
    let version = Command::new(&swiftc).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("Apple Swift version 6.4"));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-swift-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let mut mismatches = Vec::new();
    for (name, source, expected_valid) in corpus() {
        let file = root.join(format!("{name}.swift"));
        fs::write(&file, source).unwrap();
        let native = Command::new(&swiftc)
            .args(["-frontend", "-parse"])
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: Swift disagrees with the pinned corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        let report = candidate_report(&root, name, source);
        if report["recoveries"].as_array().unwrap().is_empty() != native.status.success() {
            mismatches.push(name);
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(mismatches, ["bad_param"]);
}
