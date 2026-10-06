#![cfg(all(feature = "wasm-precheck", unix))]
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn original_erlang_forms_preserve_legal_continuations_and_detect_missing_terminators() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/erlang_source_forms.json"
    ))
    .unwrap();
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-erlang-form-candidates-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut reports = Vec::new();
    for case in cases.as_array().unwrap() {
        let path = root.join(format!("{}.erl", case["name"].as_str().unwrap()));
        fs::write(&path, case["source"].as_str().unwrap()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "erlang"])
            .arg(&path)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let candidates = report["structural_observations"]
            .as_array()
            .map_or(0, Vec::len);
        if case["valid"] == true {
            assert_eq!(candidates, 0, "legal form {:?}: {report}", case["name"]);
        } else if case["name"] != "bare_expression" && case["name"] != "bare_call" {
            assert!(
                candidates > 0,
                "missing terminator {:?}: {report}",
                case["name"]
            );
            assert_eq!(report["schema_version"], "0.7.0");
            assert_eq!(
                report["structural_observations"][0]["rule_id"],
                "codeguard.erlang.form_terminator"
            );
        }
        if case["name"] == "bare_expression" || case["name"] == "bare_call" {
            assert!(
                report["recoveries"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty()),
                "bare form lost parser recovery: {report}"
            );
        }
        assert_eq!(
            report["grammar_qualified"], false,
            "case {:?}: {report}",
            case["name"]
        );
        assert_eq!(report["native"]["status"], "not_run");
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            case["source"].as_str().unwrap()
        );
        reports
            .push(serde_json::json!({"name":case["name"],"valid":case["valid"],"report":report}));
    }
    if let Ok(path) = std::env::var("CODEGUARD_ERLANG_FORM_REPORTS") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
