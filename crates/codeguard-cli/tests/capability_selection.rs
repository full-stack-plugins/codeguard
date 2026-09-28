use serde_json::Value;
use std::process::Command;

#[test]
fn planned_language_filter_is_one_explicit_gap_not_runnable() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "capabilities",
            "metal",
            "--platform=macos_arm64",
            "--category=lint",
            "--format=json",
        ])
        .output()
        .expect("run capability query");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("selection report");
    assert_eq!(report["report_type"], "capability_selection");
    assert_eq!(report["cells"].as_array().expect("cells").len(), 1);
    assert_eq!(report["cells"][0]["language"], "metal");
    assert_eq!(report["cells"][0]["legacy_status"], "planned");
    assert_eq!(report["cells"][0]["status"], "gap");
    assert!(
        report["cells"][0]["verified_combinations"]
            .as_array()
            .expect("verified combinations")
            .is_empty()
    );
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/capability-selection.schema.json"
    ))
    .expect("published query schema");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["$id"],
        "urn:codeguard:schema:capability-selection:0.2.0"
    );
}

#[test]
fn invalid_capability_dimensions_and_unknown_language_are_usage_errors() {
    for args in [
        vec!["capabilities", "metal", "--category=style", "--format=json"],
        vec![
            "capabilities",
            "metal",
            "--platform=darwin",
            "--format=json",
        ],
        vec!["capabilities", "notalanguage", "--format=json"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .expect("run invalid query");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn dependency_category_is_queryable_without_claiming_cve_coverage() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "capabilities",
            "java",
            "--platform=macos_arm64",
            "--category=dependencies",
            "--format=json",
        ])
        .output()
        .expect("run dependency capability query");
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).expect("selection report");
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["cells"].as_array().unwrap().len(), 1);
    assert_eq!(report["cells"][0]["category"], "dependencies");
    assert_eq!(report["cells"][0]["status"], "gap");
}
