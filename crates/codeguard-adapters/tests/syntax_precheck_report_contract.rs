use codeguard_adapters::parse_syntax_precheck_candidate_report;
use codeguard_core::SyntaxPrecheckStatus;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const JAVA_SHA: &str = "181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077";
const SOURCE_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

fn candidate() -> Value {
    json!({
        "schema_version": "1.0.0",
        "report_type": "syntax_precheck_candidate",
        "backend": {"kind": "bundled_tree_sitter_wasm", "language": "java", "dialect": "java", "grammar_sha256": JAVA_SHA, "grammar_abi_version": 14, "grammar_release_status": "candidate_unvalidated"},
        "scope": {"enumeration_complete": true, "cancelled": false, "excluded_files": 0},
        "files": [{"path": "src/Hello.java", "source_sha256": SOURCE_SHA, "state": {"status": "checked", "recoveries": 0, "grammar_qualified": false, "truncated": false}}],
        "precheck": {"status": "incomplete", "scope_complete": true, "cancelled": false, "selected_files": 1, "checked_files": 1, "incomplete_files": 0, "unsupported_files": 0, "unqualified_files": 1, "truncated_files": 0, "suspected_recoveries": 0},
        "native": {"status": "not_run", "reason": "native_checker_missing"},
        "delivery": {"decision": "not_evaluated", "authority": "local_unverified"}
    })
}

fn sources() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([("src/Hello.java".into(), b"hello".to_vec())])
}

fn read(report: &Value) -> Result<SyntaxPrecheckStatus, String> {
    parse_syntax_precheck_candidate_report(&serde_json::to_vec(report).unwrap(), &sources())
        .map(|outcome| outcome.status)
}

#[test]
fn candidate_asset_and_bound_source_cannot_claim_clean() {
    assert_eq!(
        read(&candidate()).unwrap(),
        SyntaxPrecheckStatus::Incomplete
    );
    let mut forged = candidate();
    forged["files"][0]["state"]["grammar_qualified"] = json!(true);
    forged["precheck"]["status"] = json!("clean");
    forged["precheck"]["unqualified_files"] = json!(0);
    assert!(read(&forged).is_err());
}

#[test]
fn rejects_unknown_major_source_drift_and_forged_aggregate() {
    let mut report = candidate();
    report["schema_version"] = json!("2.0.0");
    assert!(read(&report).is_err());
    report = candidate();
    report["files"][0]["source_sha256"] = json!("0".repeat(64));
    assert!(read(&report).is_err());
    report = candidate();
    report["precheck"]["status"] = json!("clean");
    assert!(read(&report).is_err());
}

#[test]
fn rejects_unlisted_grammar_duplicate_keys_and_extra_fields() {
    let mut report = candidate();
    report["backend"]["grammar_sha256"] = json!("0".repeat(64));
    assert!(read(&report).is_err());
    report = candidate();
    report["delivery"]["approved"] = json!(true);
    assert!(read(&report).is_err());
    let raw = serde_json::to_string(&candidate()).unwrap();
    let duplicate = raw.replacen(
        "\"schema_version\":\"1.0.0\"",
        "\"schema_version\":\"1.0.0\",\"schema_version\":\"1.0.0\"",
        1,
    );
    assert!(parse_syntax_precheck_candidate_report(duplicate.as_bytes(), &sources()).is_err());
}

#[test]
fn empty_selection_and_incomplete_scope_never_become_clean() {
    let mut report = candidate();
    report["files"] = json!([]);
    report["precheck"] = json!({"status": "not_run", "scope_complete": true, "cancelled": false, "selected_files": 0, "checked_files": 0, "incomplete_files": 0, "unsupported_files": 0, "unqualified_files": 0, "truncated_files": 0, "suspected_recoveries": 0});
    assert_eq!(read(&report).unwrap(), SyntaxPrecheckStatus::NotRun);
    report["scope"]["enumeration_complete"] = json!(false);
    report["precheck"]["status"] = json!("incomplete");
    report["precheck"]["scope_complete"] = json!(false);
    assert_eq!(read(&report).unwrap(), SyntaxPrecheckStatus::Incomplete);
}

#[test]
fn partial_scope_keeps_recoveries_without_promoting_source_violation() {
    let mut report = candidate();
    report["files"][0]["state"]["recoveries"] = json!(2);
    report["precheck"]["suspected_recoveries"] = json!(2);
    report["scope"]["enumeration_complete"] = json!(false);
    report["precheck"]["scope_complete"] = json!(false);
    assert_eq!(read(&report).unwrap(), SyntaxPrecheckStatus::Incomplete);
    report["files"][0]["state"]["unexpected"] = json!(true);
    assert!(read(&report).is_err());
}

#[test]
fn candidate_schema_declares_closed_versioned_contract() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/syntax-precheck-candidate.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], "1.0.0");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["precheck"]["$ref"], "#/$defs/precheck");
}

#[test]
fn tsx_candidate_requires_tsx_source_and_cannot_relabel_java() {
    let mut report = candidate();
    report["backend"]["language"] = json!("tsx");
    report["backend"]["dialect"] = json!("tsx");
    report["backend"]["grammar_sha256"] =
        json!("8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e");
    assert!(read(&report).is_err());

    report["files"][0]["path"] = json!("src/Component.tsx");
    let sources = BTreeMap::from([("src/Component.tsx".into(), b"hello".to_vec())]);
    let status =
        parse_syntax_precheck_candidate_report(&serde_json::to_vec(&report).unwrap(), &sources)
            .unwrap();
    assert_eq!(status.status, SyntaxPrecheckStatus::Incomplete);
}
