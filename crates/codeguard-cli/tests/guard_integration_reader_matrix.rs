use codeguard_cli::guard_integration::{
    profile::InvocationDescriptor,
    reader::read_native,
    ruff_profile::{RuffF401Policy, read_ruff_feedback},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[path = "support/guard_integration.rs"]
mod fixture;
fn raw(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tests/fixtures/guard-integration/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}
#[test]
fn six_reader_input_classes_keep_raw_identity_and_never_reuse_stale_export() {
    let complete = serde_json::to_vec(&fixture::valid()).unwrap();
    let evidence = read_native(&complete, &fixture::invocation()).unwrap();
    assert_eq!(evidence.report().finding_ids, vec!["f-1"]);
    assert_eq!(
        evidence.raw_sha256(),
        format!("{:x}", Sha256::digest(&complete))
    );
    assert_eq!(evidence.raw_bytes(), complete);
    let truncated = complete[..complete.len() - 1].to_vec();
    let duplicate = String::from_utf8(complete.clone())
        .unwrap()
        .replacen(
            "\"run_id\":\"native-001\"",
            "\"run_id\":\"native-001\",\"run_id\":\"native-001\"",
            1,
        )
        .into_bytes();
    assert_ne!(duplicate, complete);
    let oversized = vec![b' '; 1_048_577];
    let export_failed = raw("export-failed-check-feedback.json");
    for (name, bytes) in [
        ("truncated", truncated),
        ("duplicate", duplicate),
        ("oversized", oversized),
        ("unsupported-export-failed", export_failed),
    ] {
        assert!(
            read_native(&bytes, &fixture::invocation()).is_err(),
            "{name}"
        );
    }
    let mut current = serde_json::to_value(fixture::invocation()).unwrap();
    current["run_id"] = json!("new-native-run");
    let current = InvocationDescriptor::parse(&serde_json::to_vec(&current).unwrap()).unwrap();
    assert!(read_native(&complete, &current).is_err());
    assert_eq!(evidence.raw_bytes(), complete);
}
#[test]
fn actual_export_failure_keeps_native_f401_and_original_digest_but_aggregate_is_unsupported() {
    let bytes = raw("export-failed-check-feedback.json");
    let manifest: Value = serde_json::from_slice(&raw("export-failed-capture.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        manifest["stdoutSha256"].as_str().unwrap()
    );
    let native: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(native["report_type"], "check_feedback");
    assert_eq!(native["schema_version"], "0.38.0");
    assert_eq!(native["exit_code"], 3);
    assert_eq!(native["export"]["status"], "failed");
    assert_eq!(
        native["export"]["reason_code"],
        "output_existing_file_is_not_codeguard_report"
    );
    let finding = &native["native_results"]["python_lint"]["files"][0]["findings"][0];
    assert_eq!(finding["rule_id"], "F401");
    assert!(!finding["finding_id"].as_str().unwrap().is_empty());
    assert!(read_native(&bytes, &fixture::invocation()).is_err());
    let producer = manifest["producerSha256"].as_str().unwrap();
    let source = native["native_results"]["python_lint"]["files"][0]["source_sha256"]
        .as_str()
        .unwrap();
    let policy = RuffF401Policy::new("app.py", source, producer).unwrap();
    assert!(read_ruff_feedback(&bytes, &policy, "new-independent-run", 3, producer).is_err());
    // The entire actual stdout is tested. No nested report extraction or header rewrite qualifies it.
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        manifest["stdoutSha256"].as_str().unwrap()
    );
}
#[test]
fn actual_ruff_residual_report_cannot_match_a_new_capture_run() {
    let bytes = raw("ruff-f401/bad.json");
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let capture: Value = serde_json::from_slice(&raw("ruff-f401/capture.json")).unwrap();
    let producer = capture["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        capture["cases"]["bad"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let evidence = read_ruff_feedback(
        &bytes,
        &policy,
        document["run_id"].as_str().unwrap(),
        3,
        producer,
    )
    .unwrap();
    assert!(evidence.finding_count() > 0);
    assert!(read_ruff_feedback(&bytes, &policy, "different-current-run", 3, producer).is_err());
    assert_eq!(evidence.raw_bytes(), bytes);
}

#[test]
fn qualified_ruff_scope_cannot_be_replaced_by_unrelated_success() {
    use guardengine::Completeness;
    let bytes = raw("ruff-f401/clean.json");
    let baseline: Value = serde_json::from_slice(&bytes).unwrap();
    let capture: Value = serde_json::from_slice(&raw("ruff-f401/capture.json")).unwrap();
    let producer = capture["codeguardSha256"].as_str().unwrap();
    let policy = RuffF401Policy::new(
        "app.py",
        capture["cases"]["clean"]["sourceSha256"].as_str().unwrap(),
        producer,
    )
    .unwrap();
    let run = baseline["run_id"].as_str().unwrap();
    assert_eq!(
        read_ruff_feedback(&bytes, &policy, run, 3, producer)
            .unwrap()
            .completeness(),
        Completeness::Complete
    );
    for dimension in [
        "path",
        "source_sha256",
        "config_sha256",
        "tool_sha256",
        "rule_settings",
    ] {
        let mut missing = baseline.clone();
        missing["files"][0]
            .as_object_mut()
            .unwrap()
            .remove(dimension);
        assert!(
            read_ruff_feedback(
                &serde_json::to_vec(&missing).unwrap(),
                &policy,
                run,
                3,
                producer
            )
            .is_err(),
            "{dimension}"
        );
        let mut unrelated = baseline["files"][0].clone();
        unrelated["path"] = json!("unrelated.py");
        missing["files"].as_array_mut().unwrap().push(unrelated);
        assert!(
            read_ruff_feedback(
                &serde_json::to_vec(&missing).unwrap(),
                &policy,
                run,
                3,
                producer
            )
            .is_err(),
            "extra success rescued {dimension}"
        );
    }
    let mut disabled = baseline.clone();
    disabled["files"][0]["rule_settings"]["globally_enabled_mapped_rules"] = json!([]);
    assert_eq!(
        read_ruff_feedback(
            &serde_json::to_vec(&disabled).unwrap(),
            &policy,
            run,
            3,
            producer
        )
        .unwrap()
        .completeness(),
        Completeness::Partial
    );
}
