#![cfg(all(feature = "wasm-precheck", unix))]
use codeguard_cli::grammar_evaluation::replay_corpus_with_structures;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn corpus() -> Vec<u8> {
    let mut corpus: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    corpus["manifest_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    serde_json::to_vec(&corpus).unwrap()
}

#[test]
fn expired_and_cancelled_combined_replays_keep_all_cases_unknown() {
    for cancelled in [false, true] {
        let report = replay_corpus_with_structures(
            Path::new(env!("CARGO_BIN_EXE_codeguard")),
            &corpus(),
            Instant::now(),
            &AtomicBool::new(cancelled),
        )
        .unwrap();
        assert_eq!(report["sample_count"], 358);
        assert_eq!(report["cohort_count"], 35);
        assert_eq!(report["grammar_qualified_count"], 0);
        assert!(
            report["combined_cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["classification"] == "unknown"
                    && r["structural_observations"].as_array().unwrap().is_empty())
        );
        assert!(
            report["combined_cohorts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["tp"] == 0 && r["fp"] == 0 && r["fn"] == 0 && r["tn"] == 0)
        );
    }
}

#[test]
#[ignore = "full 358-case combined replay; explicitly run sequentially"]
fn full_combined_replay_keeps_raw_erlang_gaps_and_measures_structure_recovery() {
    let input = corpus();
    let report = replay_corpus_with_structures(
        Path::new(env!("CARGO_BIN_EXE_codeguard")),
        &input,
        Instant::now() + Duration::from_secs(1200),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["program_stable"], true);
    assert_eq!(report["sample_count"], 358);
    assert_eq!(report["cohort_count"], 35);
    let raw = report["raw_report"]["cases"].as_array().unwrap();
    let combined = report["combined_cases"].as_array().unwrap();
    assert!(raw.iter().all(|r| r["attempted"] == true));
    let erlang_gaps: Vec<_> = raw
        .iter()
        .filter(|r| {
            r["language"] == "erlang"
                && r["label"] == "regression"
                && r["expected_valid"] == false
                && r["classification"] == "valid"
        })
        .collect();
    assert_eq!(erlang_gaps.len(), 10);
    for row in erlang_gaps {
        let current = combined.iter().find(|r| r["id"] == row["id"]).unwrap();
        assert_eq!(current["classification"], "invalid", "{current}");
        assert!(
            current["structural_observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["rule_id"] == "codeguard.erlang.form_terminator")
        );
    }
    assert_eq!(report["grammar_qualified_count"], 0);
    if let Ok(directory) = std::env::var("CODEGUARD_COMBINED_REPORT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            Path::new(&directory).join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        std::fs::write(Path::new(&directory).join("corpus.json"), input).unwrap();
    }
}

#[test]
fn changed_program_withdraws_both_raw_and_combined_classifications() {
    use std::os::unix::fs::PermissionsExt;
    let mut input: Value = serde_json::from_slice(&corpus()).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    input["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|r| seen.insert(r["language"].as_str().unwrap().to_owned()));
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-combined-program-change-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let tool = root.join("worker");
    std::fs::write(
        &tool,
        "#!/bin/sh\nprintf '#!/bin/sh\\nexit 0\\n' > \"$0\"\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let report = replay_corpus_with_structures(
        &tool,
        &serde_json::to_vec(&input).unwrap(),
        Instant::now() + Duration::from_secs(30),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["program_stable"], false);
    for rows in [&report["raw_report"]["cases"], &report["combined_cases"]] {
        assert!(
            rows.as_array()
                .unwrap()
                .iter()
                .all(|r| r["classification"] == "unknown")
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn archived_full_report_binds_sources_rules_and_separate_group_counts() {
    let bytes = include_bytes!(
        "../../../tests/acceptance/evidence/grammar-combined-corpus-2026-10-06.json"
    );
    let input = codeguard_cli::grammar_evaluation::validate_corpus(bytes).unwrap();
    let report: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/grammar-combined-report-2026-10-06.json"
    ))
    .unwrap();
    assert_eq!(
        report["corpus_sha256"],
        format!("{:x}", Sha256::digest(bytes))
    );
    assert_eq!(report["manifest_sha256"], input.manifest_sha256);
    assert_eq!(
        report["program_sha256"],
        report["raw_report"]["program_sha256"]
    );
    assert_eq!(report["grammar_qualified_count"], 0);
    let raw = report["raw_report"]["cases"].as_array().unwrap();
    let combined = report["combined_cases"].as_array().unwrap();
    assert_eq!(combined.len(), input.cases.len());
    assert_eq!(raw.len(), combined.len());
    for ((r, c), source) in raw.iter().zip(combined).zip(&input.cases) {
        for key in [
            "id",
            "language",
            "cohort",
            "label",
            "expected_valid",
            "source_sha256",
        ] {
            assert_eq!(r[key], c[key]);
        }
        assert_eq!(c["id"], source.id);
        assert_eq!(c["source_sha256"], source.source_sha256);
        let observations = c["structural_observations"].as_array().unwrap();
        for observation in observations {
            let fact: codeguard_cli::syntax_worker_structure::SyntaxWorkerStructure =
                serde_json::from_value(observation.clone()).unwrap();
            assert!(fact.valid(&source.language, source.source.as_bytes()));
        }
        let expected = if r["classification"] == "unknown" {
            "unknown"
        } else if r["classification"] == "invalid" || !observations.is_empty() {
            "invalid"
        } else {
            "valid"
        };
        assert_eq!(c["classification"], expected);
    }
    for group in report["combined_cohorts"].as_array().unwrap() {
        let local: Vec<_> = combined
            .iter()
            .filter(|r| r["language"] == group["language"] && r["cohort"] == group["cohort"])
            .collect();
        assert_eq!(group["sample_count"].as_u64().unwrap(), local.len() as u64);
        for (key, valid, class) in [
            ("tp", false, "invalid"),
            ("fp", true, "invalid"),
            ("fn", false, "valid"),
            ("tn", true, "valid"),
        ] {
            assert_eq!(
                group[key].as_u64().unwrap(),
                local
                    .iter()
                    .filter(|r| r["label"] == "regression"
                        && r["expected_valid"] == valid
                        && r["classification"] == class)
                    .count() as u64
            );
        }
        assert_eq!(
            group["unknown_count"].as_u64().unwrap(),
            local
                .iter()
                .filter(|r| r["classification"] == "unknown")
                .count() as u64
        );
        assert_eq!(
            group["pending_label_count"].as_u64().unwrap(),
            local.iter().filter(|r| r["label"] == "pending").count() as u64
        );
    }
}
