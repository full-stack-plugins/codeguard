use codeguard_cli::grammar_evaluation::{classify_probe, validate_corpus};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn corpus() -> Value {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../grammars/manifest.json")).unwrap();
    let cases: Vec<Value> = manifest["assets"].as_array().unwrap().iter().map(|asset| {
        json!({"id":format!("{}-control",asset["language"].as_str().unwrap()),
            "language":asset["language"],"source":"x","source_sha256":format!("{:x}",Sha256::digest(b"x")),
            "expected_valid":true,"label":"pending","origin":"tests/pending_control"})
    }).collect();
    json!({"schema_version":"0.1.0","corpus_type":"grammar_regression",
        "manifest_sha256":format!("{:x}",Sha256::digest(include_bytes!("../../../grammars/manifest.json"))),
        "cases":cases})
}

fn validate(value: &Value) -> bool {
    validate_corpus(&serde_json::to_vec(value).unwrap()).is_ok()
}

#[test]
fn corpus_freezes_every_bundled_language_and_source_before_execution() {
    let good = corpus();
    assert!(validate(&good));
    let mut bad = good.clone();
    bad["cases"].as_array_mut().unwrap().pop();
    assert!(!validate(&bad));
    let mut bad = good.clone();
    bad["cases"][0]["source"] = json!("different");
    assert!(!validate(&bad));
    let mut bad = good.clone();
    bad["manifest_sha256"] = json!("0".repeat(64));
    assert!(!validate(&bad));
    let mut bad = good.clone();
    bad["cases"][0]["language"] = json!("not_supported");
    assert!(!validate(&bad));
    let mut bad = good.clone();
    bad["cases"][0]["label"] = json!("approved_holdout");
    assert!(!validate(&bad));
    let mut bad = good.clone();
    let duplicate = bad["cases"][0].clone();
    bad["cases"].as_array_mut().unwrap().push(duplicate);
    assert!(!validate(&bad));
    assert!(validate_corpus(br#"{"schema_version":"0.1.0","schema_version":"0.1.0"}"#).is_err());
}

#[test]
fn hidden_or_unavailable_recovery_never_looks_like_valid_source() {
    assert_eq!(classify_probe(0, true), None);
    assert_eq!(classify_probe(2, true), None);
    assert_eq!(classify_probe(0, false), Some(true));
    assert_eq!(classify_probe(2, false), Some(false));
}

#[test]
fn checked_in_regression_corpus_includes_all_languages_and_known_gaps() {
    let bytes = include_bytes!("../../../tests/fixtures/grammar_regression.json");
    validate_corpus(bytes).unwrap();
    let doc: Value = serde_json::from_slice(bytes).unwrap();
    for (language, id) in [
        ("erlang", "erlang-missing_period"),
        ("vbnet", "vbnet-unindented_method"),
        ("cfquery", "cfquery-missing_select_list"),
        ("kotlin", "kotlin-missing_parameter_type"),
        ("swift", "swift-bad_param"),
    ] {
        assert!(
            doc["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == id && c["language"] == language)
        );
    }
}

#[test]
fn archived_actual_report_preserves_denominators_and_pending_labels() {
    let report: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/grammar-regression-2026-10-04.json"
    ))
    .unwrap();
    let rows = report["languages"].as_array().unwrap();
    assert_eq!(
        rows.len() as u64,
        report["language_count"].as_u64().unwrap()
    );
    assert_eq!(
        report["cases"].as_array().unwrap().len() as u64,
        report["sample_count"].as_u64().unwrap()
    );
    for row in rows {
        let local: Vec<&Value> = report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["language"] == row["language"])
            .collect();
        assert_eq!(row["sample_count"].as_u64().unwrap(), local.len() as u64);
        assert_eq!(
            row["unknown_count"].as_u64().unwrap(),
            local
                .iter()
                .filter(|c| c["classification"] == "unknown")
                .count() as u64
        );
        assert_eq!(
            row["pending_label_count"].as_u64().unwrap(),
            local.iter().filter(|c| c["label"] == "pending").count() as u64
        );
        let eligible: Vec<&&Value> = local
            .iter()
            .filter(|c| c["label"] == "regression" && c["classification"] != "unknown")
            .collect();
        assert_eq!(
            row["evaluated_count"].as_u64().unwrap(),
            eligible.len() as u64
        );
        for (key, valid, classification) in [
            ("tp", false, "invalid"),
            ("fp", true, "invalid"),
            ("fn", false, "valid"),
            ("tn", true, "valid"),
        ] {
            assert_eq!(
                row[key].as_u64().unwrap(),
                eligible
                    .iter()
                    .filter(
                        |c| c["expected_valid"] == valid && c["classification"] == classification
                    )
                    .count() as u64
            );
        }
        if row["tp"] == 0 && row["fp"] == 0 {
            assert!(row["precision"].is_null());
        }
        assert_eq!(row["grammar_qualified"], false);
    }
    assert_eq!(report["authority"], "repository_regression_only");
    assert_eq!(report["native_oracle_executed"], false);
    assert_eq!(report["independent_holdout"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
fn cancelled_and_expired_replays_retain_the_entire_corpus_as_unknown() {
    use codeguard_cli::grammar_evaluation::replay_corpus;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let bytes = include_bytes!("../../../tests/fixtures/grammar_regression.json");
    for (cancelled, deadline, reason) in [
        (false, Instant::now(), "request_deadline_exceeded"),
        (
            true,
            Instant::now() + Duration::from_secs(60),
            "request_cancelled",
        ),
    ] {
        let report = replay_corpus(
            Path::new(env!("CARGO_BIN_EXE_codeguard")),
            bytes,
            deadline,
            &AtomicBool::new(cancelled),
        )
        .unwrap();
        assert_eq!(report["sample_count"], 186);
        assert_eq!(report["language_count"], 32);
        assert_eq!(report["grammar_qualified_count"], 0);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        for row in report["cases"].as_array().unwrap() {
            assert_eq!(row["classification"], "unknown");
            assert_eq!(row["reason"], reason);
            assert_eq!(row["attempted"], false);
        }
        for row in report["languages"].as_array().unwrap() {
            assert_eq!(row["unknown_count"], row["sample_count"]);
            assert_eq!(row["fp"], 0);
            assert_eq!(row["fn"], 0);
            assert_eq!(row["precision"], Value::Null);
            assert_eq!(row["fixture_outcome"], "insufficient_evidence");
        }
    }
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
fn changed_worker_identity_withdraws_all_local_classifications() {
    use codeguard_cli::grammar_evaluation::replay_corpus;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-eval-change-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let tool = root.join("worker");
    fs::write(
        &tool,
        "#!/bin/sh\nprintf '#!/bin/sh\\nexit 0\\n' > \"$0\"\nexit 0\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let report = replay_corpus(
        &tool,
        &serde_json::to_vec(&corpus()).unwrap(),
        Instant::now() + Duration::from_secs(30),
        &AtomicBool::new(false),
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(report["program_stable"], false);
    for row in report["cases"].as_array().unwrap() {
        assert_eq!(row["classification"], "unknown");
        assert_eq!(row["reason"], "grammar_evaluation_program_changed");
    }
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
#[ignore = "full fixed 186-case replay, run explicitly and sequentially with WASM"]
fn replay_every_bundled_language_and_archive_current_evidence() {
    use codeguard_cli::grammar_evaluation::replay_corpus;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let report = replay_corpus(
        Path::new(env!("CARGO_BIN_EXE_codeguard")),
        include_bytes!("../../../tests/fixtures/grammar_regression.json"),
        Instant::now() + Duration::from_secs(600),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 186);
    assert_eq!(report["program_stable"], true);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert_eq!(report["independent_holdout"], false);
    assert_eq!(report["native_oracle_executed"], false);
    let cases = report["cases"].as_array().unwrap();
    for row in cases {
        assert_eq!(row["attempted"], true, "{row}");
        assert!(
            row["reason"].is_null() || row["reason"] == "syntax_recovery_incomplete",
            "{row}"
        );
    }
    // 不把已知缺陷断言成正确行为；回放输出保留差异，资格和批准另行验收。
    assert_eq!(
        report["languages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["sample_count"].as_u64().unwrap())
            .sum::<u64>(),
        186
    );
    let pending = cases
        .iter()
        .find(|c| c["id"] == "cfquery-missing_select_list")
        .unwrap();
    assert_eq!(pending["label"], "pending");
    println!("GRAMMAR_EVALUATION_REPORT={report}");
}
