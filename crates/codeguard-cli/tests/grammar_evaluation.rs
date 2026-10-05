use codeguard_cli::grammar_evaluation::{
    classify_probe, validate_corpus, validate_corpus_against_manifest,
};
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
fn versioned_cohorts_are_explicit_and_cannot_claim_approved_holdout() {
    let legacy = corpus();
    assert!(validate(&legacy));
    let mut modern = legacy.clone();
    modern["schema_version"] = json!("0.2.0");
    for case in modern["cases"].as_array_mut().unwrap() {
        case["cohort"] = json!("upstream_grammar_regression");
    }
    assert!(validate(&modern));
    let mut bad = modern.clone();
    bad["cases"][0]["cohort"] = json!("approved_native_holdout");
    assert!(!validate(&bad));
    let mut bad = modern.clone();
    bad["cases"][0].as_object_mut().unwrap().remove("cohort");
    assert!(!validate(&bad));
    let mut bad = legacy;
    bad["cases"][0]["cohort"] = json!("repository_regression");
    assert!(!validate(&bad));
}

#[test]
fn checked_in_regression_corpus_includes_all_languages_and_known_gaps() {
    let bytes = include_bytes!("../../../tests/fixtures/grammar_regression.json");
    validate_corpus_against_manifest(bytes, ARCHIVED_MANIFEST).unwrap();
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
fn expanded_corpus_preserves_each_language_and_upstream_label_provenance() {
    let corpus = validate_corpus_against_manifest(
        include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json"),
        ARCHIVED_MANIFEST,
    )
    .unwrap();
    assert_eq!(corpus.schema_version, "0.2.0");
    assert_eq!(corpus.cases.len(), 358);
    let languages: std::collections::BTreeSet<&str> = corpus
        .cases
        .iter()
        .map(|case| case.language.as_str())
        .collect();
    assert_eq!(languages.len(), 32);
    for language in languages {
        let cases: Vec<_> = corpus
            .cases
            .iter()
            .filter(|c| c.language == language)
            .collect();
        assert!(cases.iter().any(|c| c.expected_valid), "{language}");
        assert!(cases.iter().any(|c| !c.expected_valid), "{language}");
    }
    let dart: Vec<_> = corpus
        .cases
        .iter()
        .filter(|c| c.cohort == "upstream_grammar_regression")
        .collect();
    assert_eq!(dart.len(), 150);
    assert_eq!(dart.iter().filter(|c| !c.expected_valid).count(), 4);
    assert!(dart.iter().all(|c| c.language == "dart"
        && c.label == "regression"
        && c.origin.starts_with("grammars/dart/corpus/")
        && c.origin.contains("#sha256=")));
    let pending: Vec<_> = corpus
        .cases
        .iter()
        .filter(|c| c.label == "pending")
        .collect();
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|c| c.cohort == "provisional_syntax"));
    assert!(pending.iter().any(|c| c.language == "cobol"));
}

fn assert_separate_cohort_counts(report: &Value) {
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["sample_count"], 358);
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["cohort_count"], 35);
    assert_eq!(
        report["cohort_policy"],
        "separate_sources_no_pooled_precision"
    );
    let rows = report["cases"].as_array().unwrap();
    let languages = report["languages"].as_array().unwrap();
    assert_eq!(rows.len(), 358);
    assert_eq!(languages.len(), 32);
    assert_eq!(
        languages
            .iter()
            .map(|l| l["cohorts"].as_array().unwrap().len())
            .sum::<usize>(),
        35
    );
    assert_eq!(
        languages
            .iter()
            .map(|l| l["sample_count"].as_u64().unwrap())
            .sum::<u64>(),
        358
    );
    for language in languages {
        let groups = language["cohorts"].as_array().unwrap();
        assert!(!groups.is_empty());
        if groups.len() > 1 {
            assert_eq!(language["metric_aggregation"], "not_pooled");
            assert!(language["precision"].is_null());
            assert!(language["recall"].is_null());
        } else {
            assert_eq!(language["metric_aggregation"], "single_cohort");
            assert_eq!(language["precision"], groups[0]["precision"]);
        }
        for group in groups {
            let local: Vec<_> = rows
                .iter()
                .filter(|c| c["language"] == language["language"] && c["cohort"] == group["cohort"])
                .collect();
            assert_eq!(group["sample_count"].as_u64().unwrap(), local.len() as u64);
            let unknown = local
                .iter()
                .filter(|c| c["classification"] == "unknown")
                .count();
            assert_eq!(group["unknown_count"].as_u64().unwrap(), unknown as u64);
            assert_eq!(
                group["decidable_count"].as_u64().unwrap(),
                (local.len() - unknown) as u64
            );
            assert_eq!(
                group["selected_valid_count"].as_u64().unwrap(),
                local.iter().filter(|c| c["expected_valid"] == true).count() as u64
            );
            assert_eq!(
                group["selected_invalid_count"].as_u64().unwrap(),
                local
                    .iter()
                    .filter(|c| c["expected_valid"] == false)
                    .count() as u64
            );
            assert_eq!(
                group["pending_label_count"].as_u64().unwrap(),
                local.iter().filter(|c| c["label"] == "pending").count() as u64
            );
            let eligible: Vec<_> = local
                .iter()
                .filter(|c| c["label"] == "regression" && c["classification"] != "unknown")
                .collect();
            assert_eq!(
                group["evaluated_count"].as_u64().unwrap(),
                eligible.len() as u64
            );
            for (key, valid, classification) in [
                ("tp", false, "invalid"),
                ("fp", true, "invalid"),
                ("fn", false, "valid"),
                ("tn", true, "valid"),
            ] {
                assert_eq!(
                    group[key].as_u64().unwrap(),
                    eligible
                        .iter()
                        .filter(|c| c["expected_valid"] == valid
                            && c["classification"] == classification)
                        .count() as u64
                );
            }
            assert_eq!(group["grammar_qualified"], false);
        }
        for key in [
            "sample_count",
            "selected_valid_count",
            "selected_invalid_count",
            "decidable_count",
            "unknown_count",
            "pending_label_count",
            "evaluated_count",
            "tp",
            "fp",
            "fn",
            "tn",
        ] {
            assert_eq!(
                language[key].as_u64().unwrap(),
                groups.iter().map(|c| c[key].as_u64().unwrap()).sum::<u64>()
            );
        }
    }
    assert_eq!(report["native_oracle_executed"], false);
    assert_eq!(report["independent_holdout"], false);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}

#[test]
fn archived_expanded_report_binds_cases_and_preserves_cohort_denominators() {
    let report: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/grammar-cohorts-2026-10-04.json"
    ))
    .unwrap();
    assert_separate_cohort_counts(&report);
    let bytes = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    let corpus = validate_corpus_against_manifest(bytes, ARCHIVED_MANIFEST).unwrap();
    assert_eq!(
        report["corpus_sha256"],
        format!("{:x}", Sha256::digest(bytes))
    );
    assert_eq!(report["manifest_sha256"], corpus.manifest_sha256);
    for (actual, expected) in report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&corpus.cases)
    {
        assert_eq!(actual["id"], expected.id);
        assert_eq!(actual["source_sha256"], expected.source_sha256);
        assert_eq!(actual["cohort"], expected.cohort);
        assert_eq!(actual["label"], expected.label);
        assert_eq!(actual["expected_valid"], expected.expected_valid);
    }
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
fn expired_cohort_replay_preserves_label_coverage_without_pooling() {
    let report = codeguard_cli::grammar_evaluation::replay_corpus(
        std::path::Path::new(env!("CARGO_BIN_EXE_codeguard")),
        &current_corpus_bytes(include_bytes!(
            "../../../tests/fixtures/grammar_regression_v0_2.json"
        )),
        std::time::Instant::now(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_separate_cohort_counts(&report);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["classification"] == "unknown" && c["attempted"] == false)
    );
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
#[ignore = "full fixed 358-case replay, run explicitly and sequentially with WASM"]
fn replay_expanded_cohorts_and_archive_current_evidence() {
    let report = codeguard_cli::grammar_evaluation::replay_corpus(
        std::path::Path::new(env!("CARGO_BIN_EXE_codeguard")),
        &current_corpus_bytes(include_bytes!(
            "../../../tests/fixtures/grammar_regression_v0_2.json"
        )),
        std::time::Instant::now() + std::time::Duration::from_secs(1200),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_separate_cohort_counts(&report);
    assert_eq!(report["program_stable"], true);
    for row in report["cases"].as_array().unwrap() {
        assert_eq!(row["attempted"], true, "{row}");
        assert!(
            row["reason"].is_null() || row["reason"] == "syntax_recovery_incomplete",
            "{row}"
        );
    }
    println!("GRAMMAR_COHORT_EVALUATION_REPORT={report}");
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
    let bytes = current_corpus_bytes(include_bytes!(
        "../../../tests/fixtures/grammar_regression.json"
    ));
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
            &bytes,
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
        &current_corpus_bytes(include_bytes!(
            "../../../tests/fixtures/grammar_regression.json"
        )),
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

const ARCHIVED_MANIFEST: &[u8] =
    include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_04.json");

fn current_corpus_bytes(archived: &[u8]) -> Vec<u8> {
    validate_corpus_against_manifest(archived, ARCHIVED_MANIFEST).unwrap();
    let mut value: Value = serde_json::from_slice(archived).unwrap();
    let cases = value["cases"].clone();
    value["manifest_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    assert_eq!(value["cases"], cases);
    let bytes = serde_json::to_vec(&value).unwrap();
    validate_corpus(&bytes).unwrap();
    bytes
}

#[test]
fn archived_manifest_binding_does_not_authorize_current_replay_identity() {
    let bytes = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    let historical = validate_corpus_against_manifest(bytes, ARCHIVED_MANIFEST).unwrap();
    assert_eq!(
        historical.manifest_sha256,
        format!("{:x}", Sha256::digest(ARCHIVED_MANIFEST))
    );
    assert!(validate_corpus(bytes).is_err());
    assert!(validate_corpus_against_manifest(bytes, b"{}").is_err());
    let rebound = current_corpus_bytes(bytes);
    assert_ne!(Sha256::digest(bytes), Sha256::digest(&rebound));
    let a: Value = serde_json::from_slice(bytes).unwrap();
    let b: Value = serde_json::from_slice(&rebound).unwrap();
    assert_eq!(a["cases"], b["cases"]);
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
fn archived_corpus_cannot_start_current_worker_without_explicit_rebinding() {
    let result = codeguard_cli::grammar_evaluation::replay_corpus(
        std::path::Path::new("/definitely-absent-worker"),
        include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json"),
        std::time::Instant::now(),
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert_eq!(
        result.unwrap_err(),
        "grammar_evaluation_corpus_identity_invalid"
    );
}

#[test]
fn historical_manifest_shape_and_digest_remain_checked() {
    let corpus = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    let mut manifest: Value = serde_json::from_slice(ARCHIVED_MANIFEST).unwrap();
    manifest["assets"][0]["language"] = json!("renamed");
    assert!(
        validate_corpus_against_manifest(corpus, &serde_json::to_vec(&manifest).unwrap()).is_err()
    );
    let duplicate = br#"{"assets":[],"assets":[]}"#;
    assert!(validate_corpus_against_manifest(corpus, duplicate).is_err());
    assert!(validate_corpus_against_manifest(corpus, &vec![b' '; 1024 * 1024 + 1]).is_err());
}

#[test]
fn current_manifest_replay_archive_preserves_all_samples_and_distinct_identity() {
    let input =
        include_bytes!("../../../tests/acceptance/evidence/grammar-current-corpus-2026-10-05.json");
    let manifest =
        include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_05.json");
    let corpus = validate_corpus_against_manifest(input, manifest).unwrap();
    let report: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/grammar-current-manifest-2026-10-05.json"
    ))
    .unwrap();
    assert_separate_cohort_counts(&report);
    assert_eq!(
        report["corpus_sha256"],
        format!("{:x}", Sha256::digest(input))
    );
    assert_eq!(report["manifest_sha256"], corpus.manifest_sha256);
    assert_eq!(report["program_stable"], true);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|case| case["attempted"] == true)
    );
    let historical: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    let current: Value = serde_json::from_slice(input).unwrap();
    assert_eq!(current["cases"], historical["cases"]);
    assert_ne!(current["manifest_sha256"], historical["manifest_sha256"]);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert_eq!(report["delivery_decision"], "not_evaluated");
}
