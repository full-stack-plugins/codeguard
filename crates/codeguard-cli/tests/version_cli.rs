use serde_json::Value;
use std::collections::BTreeSet;
use std::process::Command;

#[test]
fn version_json_reports_protocol_without_claiming_unverified_release_identity() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["--version", "--format", "json"])
        .output()
        .expect("运行版本查询");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("纯净 JSON");
    assert_eq!(report["report_type"], "version");
    assert_eq!(report["cli_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["check_protocol_major"], 1);
    assert_eq!(report["rulepack_compatibility"], "unverified");
    assert!(report["build_identity"].is_null());
    assert!(
        report["target"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/version-report.schema.json"))
            .expect("版本报告 schema");
    let required: BTreeSet<_> = schema["required"]
        .as_array()
        .expect("必需字段")
        .iter()
        .map(|field| field.as_str().expect("字段名"))
        .collect();
    let actual: BTreeSet<_> = report
        .as_object()
        .expect("报告对象")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(required, actual);
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn invalid_version_options_fail_before_any_output() {
    for args in [
        vec!["--version", "--format", "yaml"],
        vec!["--version", "--timeout", "1s"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .output()
            .expect("运行错误参数");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
