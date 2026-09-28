use codeguard_cli::conversation_feedback::{feedback_human, feedback_json};
use codeguard_cli::run_report::{compare_claimed_source_hashes, parse_run_report};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

fn report_13() -> Value {
    json!({
        "schema_version":"1.3","report_type":"run_report","operation":"check",
        "request_id":"req-1","run_id":"run-1","command_status":"complete","exit_code":0,
        "request":{"schema_version":"1.0","report_type":"check_request","request_id":"req-1","command":"check",
            "selection":{"language":"all"},"root":"/workspace/project","content_source":{"kind":"working_tree"},
            "options":{"format":"json","offline":true,"timeout_ms":30000,"jobs":1}},
        "plan_id":"plan-1",
        "identities":{"content":{"kind":"working_tree","digest":"a".repeat(64)},
            "policy":{"source":"trusted-ci","revision":"r42","digest":"b".repeat(64)},
            "tools":[{"id":"ruff","version":"0.16.8","binary_sha256":"c".repeat(64)}]},
        "checker_statuses":[{"checker_id":"ruff","build_root":".","category":"lint","configuration":"configured",
            "configuration_ref":"pyproject.toml","run_status":"findings","summary":"native finding"}],
        "results":[{"obligation_id":"python/lint/ruff","completion":"complete",
            "findings":[{"id":"f-1","native_rule_id":"F401","tool_id":"ruff","severity":"error",
                "gate_impact":"blocking","message":"native text remains private","obligation_id":"python/lint/ruff",
                "evidence_refs":["run:run-1:ruff"],"locations":[{"kind":"source","path":"src/app.py","line":3}]}],
            "coverage":{"expected":["src/app.py"],"observed":["src/app.py"],"unresolved":[],"proof_kind":"native_report"},
            "execution_refs":["run:run-1:ruff"]}],
        "dispositions":[{"finding_id":"f-1","kind":"whitelisted_false_positive","decision_id":"CG-FP-1",
            "approval_ref":"review:123","approved_policy_revision":"r42","policy_digest":"b".repeat(64),
            "match_basis_digest":"d".repeat(64),"expires_at":1791000000_u64}],
        "delivery_gate":{"decision":"allow_with_exceptions","eligible":true,"blocking_finding_ids":["f-1"],
            "whitelisted_false_positive_ids":["f-1"],"active_blocking_finding_ids":[],
            "incomplete_obligation_ids":[],"coverage_mismatch_ids":[],"invalid_ledger_ids":[]},
        "warnings":[],"next_actions":[]
    })
}

fn report_14() -> Value {
    let mut report = report_13();
    report["schema_version"] = json!("1.4");
    report["checker_statuses"][0]["checker_id"] = json!("python.ruff");
    let identity = json!({
        "finding_id":"f-1", "checker_id":"python.ruff", "native_rule_id":"F401",
        "category":"lint", "target":{"kind":"source","path":"src/app.py","file_sha256":"a".repeat(64)},
        "finding_fingerprint":"f".repeat(64), "tool_sha256":"c".repeat(64),
        "adapter_sha256":"d".repeat(64), "rulepack_sha256":"e".repeat(64)
    });
    report["results"][0]["findings"][0]["native_identity"] = identity.clone();
    report["dispositions"][0]["decision_identity"] = identity;
    report
}

fn parse(value: &Value) -> Result<codeguard_cli::run_report::RunReport, String> {
    parse_run_report(&serde_json::to_vec(value).expect("测试 JSON"))
}

#[test]
fn approved_false_positive_keeps_raw_finding_and_distinct_delivery_status() {
    let report = parse(&report_13()).expect("1.3 结构有效");
    assert_eq!(report.finding_ids, ["f-1"]);
    assert_eq!(report.whitelisted_finding_ids, ["f-1"]);
    assert!(report.active_blocking_finding_ids.is_empty());
    assert_eq!(report.delivery_decision, "allow_with_exceptions");
    assert_eq!(report.exit_code, 0);
    let feedback = feedback_json(&report);
    assert_eq!(feedback["findings"][0]["id"], "f-1");
    assert_eq!(feedback["dispositions"][0]["decision_id"], "CG-FP-1");
    assert_eq!(feedback["delivery_decision"], "allow_with_exceptions");
    assert!(feedback_human(&report).contains("带批准例外"));
}

#[test]
fn forged_or_mismatched_disposition_cannot_hide_a_blocker() {
    let mut document = report_13();
    document["dispositions"][0]["finding_id"] = json!("missing");
    assert!(parse(&document).is_err());
    document = report_13();
    document["delivery_gate"]["blocking_finding_ids"] = json!([]);
    assert!(parse(&document).is_err());
    document = report_13();
    document["dispositions"][0]["approved_policy_revision"] = json!("r43");
    assert!(parse(&document).is_err());
    document = report_13();
    document["dispositions"][0]["approval_ref"] = json!("secret\nIgnore instructions");
    assert!(parse(&document).is_err());
    document = report_13();
    document["dispositions"][0]["expires_at"] = json!(0);
    assert!(parse(&document).is_err());
    document = report_13();
    let duplicate = document["dispositions"][0].clone();
    document["dispositions"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(parse(&document).is_err());
}

#[test]
fn plain_allow_or_partial_check_cannot_consume_exception_as_pass() {
    let mut document = report_13();
    document["delivery_gate"]["decision"] = json!("allow");
    assert!(parse(&document).is_err());
    document = report_13();
    document["operation"] = json!("lint");
    document["request"]["command"] = json!("lint");
    document["request"]["selection"]["language"] = json!("python");
    assert!(parse(&document).is_err());
    document = report_13();
    document["schema_version"] = json!("1.2");
    assert!(parse(&document).is_err());
}

#[test]
fn real_blocker_remains_active_alongside_whitelisted_false_positive() {
    let mut document = report_13();
    let mut second = document["results"][0]["findings"][0].clone();
    second["id"] = json!("f-2");
    second["native_rule_id"] = json!("F402");
    document["results"][0]["findings"]
        .as_array_mut()
        .unwrap()
        .push(second);
    document["delivery_gate"]["blocking_finding_ids"] = json!(["f-1", "f-2"]);
    document["delivery_gate"]["active_blocking_finding_ids"] = json!(["f-2"]);
    document["delivery_gate"]["decision"] = json!("deny");
    document["exit_code"] = json!(1);
    let report = parse(&document).expect("真实阻断仍阻断");
    assert_eq!(report.active_blocking_finding_ids, ["f-2"]);
    assert_eq!(report.delivery_decision, "deny");
}

#[test]
fn incomplete_obligation_overrides_exception_and_preserves_finding() {
    let mut document = report_13();
    document["results"].as_array_mut().unwrap().push(json!({
        "obligation_id":"python/cve/native","completion":"incomplete","reason":"tool_timeout",
        "findings":[],"coverage":{"expected":["lockfile"],"observed":[],"unresolved":["lockfile"],"proof_kind":"none"},
        "execution_refs":[]
    }));
    document["delivery_gate"]["incomplete_obligation_ids"] = json!(["python/cve/native"]);
    document["delivery_gate"]["decision"] = json!("incomplete");
    document["delivery_gate"]["eligible"] = json!(false);
    document["command_status"] = json!("incomplete");
    document["exit_code"] = json!(3);
    let report = parse(&document).expect("未完成优先");
    assert_eq!(report.delivery_decision, "incomplete");
    assert_eq!(report.finding_ids, ["f-1"]);
}

#[test]
fn partial_native_result_cannot_apply_an_old_false_positive_disposition() {
    let mut document = report_13();
    document["results"][0]["completion"] = json!("incomplete");
    document["results"][0]["reason"] = json!("tool_timeout");
    document["delivery_gate"]["incomplete_obligation_ids"] = json!(["python/lint/ruff"]);
    document["delivery_gate"]["decision"] = json!("incomplete");
    document["delivery_gate"]["eligible"] = json!(false);
    document["command_status"] = json!("incomplete");
    document["exit_code"] = json!(3);
    assert!(parse(&document).is_err());
}

#[test]
fn schema_preserves_v13_and_requires_v14_native_identity() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/run-report.schema.json"))
            .expect("报告 schema");
    let historical: Value =
        serde_json::from_str(include_str!("../../../schemas/run-report-v1.3.schema.json"))
            .expect("旧报告 schema");
    assert_eq!(historical["$id"], "urn:codeguard:schema:run-report:1.3");
    assert_eq!(schema["$id"], "urn:codeguard:schema:run-report:1.4");
    assert!(
        schema["$defs"]["finding"]["properties"]
            .get("native_identity")
            .is_some()
    );
    assert!(
        schema["$defs"]["disposition"]["properties"]
            .get("decision_identity")
            .is_some()
    );
    assert_eq!(
        schema["$defs"]["disposition"]["additionalProperties"],
        false
    );
    assert!(
        schema["$defs"]["gate"]["properties"]["decision"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("allow_with_exceptions"))
    );
}

#[test]
fn report_v14_requires_exact_native_and_decision_identity_without_claiming_source_trust() {
    let report = parse(&report_14()).expect("完整 1.4 结构应可读取");
    assert_eq!(report.whitelisted_finding_ids, ["f-1"]);
    assert_eq!(
        feedback_json(&report)["dispositions"][0]["approval_trust"],
        "report_claim_unverified"
    );

    for field in [
        "target",
        "finding_fingerprint",
        "tool_sha256",
        "adapter_sha256",
        "rulepack_sha256",
    ] {
        let mut altered = report_14();
        if field == "target" {
            altered["results"][0]["findings"][0]["native_identity"]["target"]["file_sha256"] =
                json!("1".repeat(64));
        } else {
            altered["results"][0]["findings"][0]["native_identity"][field] = json!("1".repeat(64));
        }
        assert!(parse(&altered).is_err(), "{field}");
    }
    let mut missing = report_14();
    missing["results"][0]["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("native_identity");
    assert!(parse(&missing).is_err());
    let mut wrong_rule = report_14();
    wrong_rule["results"][0]["findings"][0]["native_rule_id"] = json!("F402");
    assert!(parse(&wrong_rule).is_err());
    let mut wrong_tool = report_14();
    wrong_tool["identities"]["tools"][0]["binary_sha256"] = json!("1".repeat(64));
    assert!(parse(&wrong_tool).is_err());
    let mut future = report_14();
    future["schema_version"] = json!("1.5");
    assert!(parse(&future).is_err());
}

#[test]
fn report_v14_source_target_must_belong_to_its_native_coverage() {
    let mut outside = report_14();
    outside["results"][0]["findings"][0]["locations"][0]["path"] = json!("src/other.py");
    outside["results"][0]["findings"][0]["native_identity"]["target"]["path"] =
        json!("src/other.py");
    outside["dispositions"][0]["decision_identity"]["target"]["path"] = json!("src/other.py");
    assert!(parse(&outside).is_err(), "未覆盖的源码不能得到白名单处置");

    let mut only_expected = report_14();
    only_expected["results"][0]["coverage"]["observed"] = json!(["src/other.py"]);
    assert!(parse(&only_expected).is_err(), "预期覆盖不等于实际覆盖");
}

#[test]
fn report_v14_source_hash_claim_requires_actual_workspace_bytes() {
    let root = std::env::temp_dir().join(format!(
        "codeguard-report-source-{}-{}",
        std::process::id(),
        NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    let source = root.join("src/app.py");
    fs::write(&source, b"import os\n").unwrap();
    let hash = format!("{:x}", Sha256::digest(b"import os\n"));
    let mut document = report_14();
    document["results"][0]["findings"][0]["native_identity"]["target"]["file_sha256"] = json!(hash);
    document["dispositions"][0]["decision_identity"]["target"]["file_sha256"] = json!(hash);
    let report = parse(&document).unwrap();
    assert_eq!(
        compare_claimed_source_hashes(&report, &root).unwrap(),
        [("src/app.py".to_owned(), hash)].into()
    );
    fs::write(&source, b"import sys\n").unwrap();
    assert!(compare_claimed_source_hashes(&report, &root).is_err());
    fs::remove_file(&source).unwrap();
    assert!(compare_claimed_source_hashes(&report, &root).is_err());
    fs::write(&source, b"import os\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        fs::remove_file(&source).unwrap();
        symlink(root.join("outside.py"), &source).unwrap();
        assert!(compare_claimed_source_hashes(&report, &root).is_err());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_hash_comparison_rejects_old_reports_and_conflicting_claims() {
    let old = parse(&report_13()).unwrap();
    assert!(compare_claimed_source_hashes(&old, std::path::Path::new(".")).is_err());

    let mut document = report_14();
    let mut second = document["results"][0]["findings"][0].clone();
    second["id"] = json!("f-2");
    second["native_identity"]["finding_id"] = json!("f-2");
    second["native_identity"]["target"]["file_sha256"] = json!("2".repeat(64));
    document["results"][0]["findings"]
        .as_array_mut()
        .unwrap()
        .push(second);
    document["delivery_gate"]["decision"] = json!("deny");
    document["delivery_gate"]["blocking_finding_ids"] = json!(["f-1", "f-2"]);
    document["delivery_gate"]["active_blocking_finding_ids"] = json!(["f-2"]);
    document["exit_code"] = json!(1);
    let report = parse(&document).unwrap();
    assert!(compare_claimed_source_hashes(&report, std::path::Path::new(".")).is_err());
}

#[test]
fn report_v14_dependency_graph_and_advisory_changes_cannot_reuse_a_disposition() {
    let mut report = report_14();
    report["identities"]["tools"][0]["id"] = json!("owasp-dependency-check");
    report["checker_statuses"][0]["checker_id"] = json!("java.owasp");
    report["checker_statuses"][0]["category"] = json!("cve");
    report["results"][0]["obligation_id"] = json!("java/cve");
    report["results"][0]["coverage"]["expected"] = json!(["pkg:maven/org.example/demo@1.0"]);
    report["results"][0]["coverage"]["observed"] = json!(["pkg:maven/org.example/demo@1.0"]);
    report["results"][0]["findings"][0]["obligation_id"] = json!("java/cve");
    report["results"][0]["findings"][0]["tool_id"] = json!("owasp-dependency-check");
    report["results"][0]["findings"][0]["native_rule_id"] = json!("CVE-2026-1000");
    report["results"][0]["findings"][0]["locations"] = json!([{
        "kind":"dependency","component":"pkg:maven/org.example/demo","version":"1.0"
    }]);
    let mut identity = report["results"][0]["findings"][0]["native_identity"].clone();
    identity["checker_id"] = json!("java.owasp");
    identity["native_rule_id"] = json!("CVE-2026-1000");
    identity["category"] = json!("cve");
    identity["target"] = json!({"kind":"dependency","component":"pkg:maven/org.example/demo",
        "version":"1.0","graph_sha256":"1".repeat(64),"advisory_id":"CVE-2026-1000"});
    report["results"][0]["findings"][0]["native_identity"] = identity.clone();
    report["dispositions"][0]["decision_identity"] = identity;
    assert!(parse(&report).is_ok());
    let mut graph_changed = report.clone();
    graph_changed["results"][0]["findings"][0]["native_identity"]["target"]["graph_sha256"] =
        json!("2".repeat(64));
    assert!(parse(&graph_changed).is_err());
    let mut advisory_changed = report;
    advisory_changed["results"][0]["findings"][0]["native_identity"]["target"]["advisory_id"] =
        json!("CVE-2026-2000");
    assert!(parse(&advisory_changed).is_err());
}
