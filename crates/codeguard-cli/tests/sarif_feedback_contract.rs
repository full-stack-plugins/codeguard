use codeguard_cli::run_report::parse_run_report;
use codeguard_cli::sarif_feedback::feedback_sarif;
use serde_json::{Value, json};

fn base_report() -> Value {
    json!({
        "schema_version":"1.3","report_type":"run_report","operation":"check",
        "request_id":"req-sarif","run_id":"run-sarif","command_status":"complete","exit_code":0,
        "request":{"schema_version":"1.0","report_type":"check_request","request_id":"req-sarif",
            "command":"check","selection":{"language":"all"},"root":"/workspace/example",
            "content_source":{"kind":"working_tree"},
            "options":{"format":"sarif","offline":true,"timeout_ms":30000,"jobs":1}},
        "plan_id":"plan-sarif",
        "identities":{"content":{"kind":"working_tree","digest":"a".repeat(64)},
            "policy":{"source":"candidate","revision":"r1","digest":"b".repeat(64)},
            "tools":[{"id":"ruff","version":"0.16.8","binary_sha256":"c".repeat(64)}]},
        "checker_statuses":[{"checker_id":"ruff","build_root":".","category":"lint",
            "configuration":"configured","configuration_ref":"pyproject.toml",
            "run_status":"findings","summary":"token=private","next_action":"print token=private"}],
        "results":[{"obligation_id":"python/lint/ruff","completion":"complete",
            "findings":[{"id":"f-sarif","native_rule_id":"F401","tool_id":"ruff",
                "severity":"error","gate_impact":"blocking","message":"token=private",
                "obligation_id":"python/lint/ruff","evidence_refs":["private:token=private"],
                "locations":[{"kind":"source","path":"src/token=private.py","line":3}]}],
            "coverage":{"expected":["src/token=private.py"],"observed":["src/token=private.py"],
                "unresolved":[],"proof_kind":"native_report"},
            "execution_refs":["run:run-sarif:ruff"]}],
        "dispositions":[{"finding_id":"f-sarif","kind":"whitelisted_false_positive",
            "decision_id":"CG-FP-7","approval_ref":"review:7","approved_policy_revision":"r1",
            "policy_digest":"b".repeat(64),"match_basis_digest":"d".repeat(64),"expires_at":1791000000_u64}],
        "delivery_gate":{"decision":"allow_with_exceptions","eligible":true,
            "blocking_finding_ids":["f-sarif"],"whitelisted_false_positive_ids":["f-sarif"],
            "active_blocking_finding_ids":[],"incomplete_obligation_ids":[],
            "coverage_mismatch_ids":[],"invalid_ledger_ids":[]},
        "warnings":["token=private"],"next_actions":["print token=private"]
    })
}

#[test]
fn incomplete_without_findings_is_not_a_clean_sarif_run() {
    let mut source = base_report();
    source["results"][0]["completion"] = json!("incomplete");
    source["results"][0]["reason"] = json!("tool_timeout");
    source["results"][0]["findings"] = json!([]);
    source["results"][0]["coverage"] = json!({
        "expected":["src/token=private.py"],"observed":[],
        "unresolved":["src/token=private.py"],"proof_kind":"none"
    });
    source["dispositions"] = json!([]);
    source["delivery_gate"] = json!({
        "decision":"incomplete","eligible":false,"blocking_finding_ids":[],
        "whitelisted_false_positive_ids":[],"active_blocking_finding_ids":[],
        "incomplete_obligation_ids":["python/lint/ruff"],
        "coverage_mismatch_ids":[],"invalid_ledger_ids":[]
    });
    source["command_status"] = json!("incomplete");
    source["exit_code"] = json!(3);
    let report = parse_run_report(&serde_json::to_vec(&source).unwrap()).unwrap();
    let sarif = feedback_sarif(&report);
    assert_eq!(sarif["version"], "2.1.0");
    assert_eq!(sarif["runs"][0]["results"], json!([]));
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"][0]["level"],
        "error"
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "incomplete"
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["incompleteObligationCount"],
        1
    );
    assert!(!sarif.to_string().contains("token=private"));
}

#[test]
fn allowlisted_finding_remains_a_result_with_unverified_disposition() {
    let report = parse_run_report(&serde_json::to_vec(&base_report()).unwrap()).unwrap();
    let sarif = feedback_sarif(&report);
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "allow_with_exceptions"
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardSourceTrust"],
        "structure_validated_provenance_unverified"
    );
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["codeguardDisposition"],
        "whitelisted_false_positive"
    );
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["decisionId"],
        "CG-FP-7"
    );
    assert!(sarif["runs"][0]["results"][0].get("suppressions").is_none());
    assert!(!sarif.to_string().contains("token=private"));
}

#[test]
fn version_14_bound_identity_still_emits_private_unverified_sarif_result() {
    let mut source = base_report();
    source["schema_version"] = json!("1.4");
    source["checker_statuses"][0]["checker_id"] = json!("python.ruff");
    let identity = json!({
        "finding_id":"f-sarif","checker_id":"python.ruff","native_rule_id":"F401",
        "category":"lint","target":{"kind":"source","path":"src/token=private.py","file_sha256":"a".repeat(64)},
        "finding_fingerprint":"f".repeat(64),"tool_sha256":"c".repeat(64),
        "adapter_sha256":"d".repeat(64),"rulepack_sha256":"e".repeat(64)
    });
    source["results"][0]["findings"][0]["native_identity"] = identity.clone();
    source["dispositions"][0]["decision_identity"] = identity;
    let report = parse_run_report(&serde_json::to_vec(&source).unwrap()).unwrap();
    let sarif = feedback_sarif(&report);
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["codeguardApprovalTrust"],
        "report_claim_unverified"
    );
    assert!(sarif["runs"][0]["results"][0].get("suppressions").is_none());
    assert!(!sarif.to_string().contains("token=private"));
}

#[test]
fn mixed_allowlisted_and_active_findings_remain_visible_as_deny() {
    let mut source = base_report();
    source["results"][0]["findings"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"f-active","native_rule_id":"F821","tool_id":"ruff",
            "severity":"error","gate_impact":"blocking","message":"token=private",
            "obligation_id":"python/lint/ruff","evidence_refs":["private:token=private"],
            "locations":[{"kind":"source","path":"src/token=private.py","line":4}]
        }));
    source["delivery_gate"]["decision"] = json!("deny");
    source["delivery_gate"]["eligible"] = json!(true);
    source["delivery_gate"]["blocking_finding_ids"] = json!(["f-sarif", "f-active"]);
    source["delivery_gate"]["active_blocking_finding_ids"] = json!(["f-active"]);
    source["exit_code"] = json!(1);
    let report = parse_run_report(&serde_json::to_vec(&source).unwrap()).unwrap();
    let sarif = feedback_sarif(&report);
    let results = sarif["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0]["properties"]["codeguardDisposition"],
        "whitelisted_false_positive"
    );
    assert_eq!(results[1]["properties"]["codeguardDisposition"], "active");
    assert_eq!(
        sarif["runs"][0]["properties"]["codeguardDeliveryDecision"],
        "deny"
    );
    assert!(
        sarif["runs"][0]["invocations"][0]["executionSuccessful"]
            .as_bool()
            .unwrap()
    );
    assert!(!sarif.to_string().contains("token=private"));
}
