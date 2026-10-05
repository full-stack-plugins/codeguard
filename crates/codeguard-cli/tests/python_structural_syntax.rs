#![cfg(all(feature = "wasm-precheck", unix))]
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn probe_preserves_python_structure_separately_from_raw_recoveries() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-python-structure-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("app.py");
    for source in [
        "def run():\n",
        "if True:\npass\n",
        "def run():\n    # only comment\n",
    ] {
        fs::write(&path, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "python"])
            .arg(&path)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.2.0");
        assert_eq!(
            report["recoveries"],
            serde_json::json!([]),
            "cannot fabricate parser errors"
        );
        let facts = report["structural_observations"]
            .as_array()
            .expect("retain distinct structural evidence");
        assert!(!facts.is_empty());
        assert_eq!(facts[0]["rule_id"], "codeguard.python.required_suite");
        assert_eq!(facts[0]["basis"], "codeguard_structure_rule");
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
    for source in [
        "def run():\n    pass\n",
        "def run():\n    ...\n",
        "def run():\n    \"\"\"说明\"\"\"\n",
        "if True:\n    pass\n",
        "# empty module is valid\n",
    ] {
        fs::write(&path, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "python"])
            .arg(&path)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.1.0");
        assert!(report.get("structural_observations").is_none());
        assert_eq!(report["recoveries"], serde_json::json!([]));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn structural_evidence_rejects_wrong_rule_language_and_positions() {
    use codeguard_cli::syntax_worker_structure::SyntaxWorkerStructure;
    let source = "def run():\n".as_bytes();
    let row = SyntaxWorkerStructure {
        basis: "codeguard_structure_rule".into(),
        rule_id: "codeguard.python.required_suite".into(),
        rule_version: "1.0.0".into(),
        rule_sha256: codeguard_adapters::python_suite_rule_sha256(),
        parent_syntax_kind: "function_definition".into(),
        start_byte: 10,
        end_byte: 10,
        start_row: 0,
        start_column_byte: 10,
        end_row: 0,
        end_column_byte: 10,
    };
    assert!(row.valid("python", source));
    assert!(!row.valid("rust", source));
    for field in [
        "basis",
        "rule_id",
        "rule_version",
        "rule_sha256",
        "parent_syntax_kind",
    ] {
        let mut value = serde_json::to_value(&row).unwrap();
        value[field] = serde_json::json!("forged");
        let invalid: SyntaxWorkerStructure = serde_json::from_value(value).unwrap();
        assert!(!invalid.valid("python", source), "{field}");
    }
    let mut invalid = row.clone();
    invalid.end_byte = source.len() + 1;
    assert!(!invalid.valid("python", source));
    let mut invalid = row;
    invalid.start_column_byte = 9;
    assert!(!invalid.valid("python", source));
}
