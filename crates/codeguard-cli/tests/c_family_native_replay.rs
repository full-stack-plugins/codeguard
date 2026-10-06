#![cfg(all(feature = "wasm-precheck", unix))]
use codeguard_cli::grammar_native_differential::replay_native_corpus;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn corpus() -> Vec<u8> {
    let mut corpus: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    codeguard_cli::grammar_evaluation::validate_corpus_against_manifest(
        &serde_json::to_vec(&corpus).unwrap(),
        include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_04.json"),
    )
    .unwrap();
    corpus["manifest_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["language"] != "c" && c["language"] != "cpp");
    for language in ["c", "cpp"] {
        for (name, source, valid) in [
            ("plain", "int x=1;\n", true),
            ("warning", "int f(void) { int unused; return 0; }\n", true),
            ("literal", "const char *s=\"#literal\";\n", true),
            ("syntax", "int x = ;\n", false),
            ("semantic", "UnknownType x;\n", true),
            ("context", "%:include \"missing.h\"\n", true),
        ] {
            corpus["cases"].as_array_mut().unwrap().push(json!({"id":format!("{language}-replay-{name}"),"language":language,"source":source,"source_sha256":format!("{:x}",Sha256::digest(source)),"expected_valid":valid,"label":"regression","cohort":"repository_regression","origin":"crates/codeguard-cli/tests/c_family_native_replay.rs"}));
        }
    }
    serde_json::to_vec(&corpus).unwrap()
}

fn replay(tool: PathBuf, budget: u64) -> Value {
    replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &corpus(),
        &BTreeMap::from([("c".into(), tool.clone()), ("cpp".into(), tool)]),
        Instant::now() + Duration::from_secs(budget),
        &AtomicBool::new(false),
    )
    .expect("C11/C++17 must use explicit native Clang observers")
}

#[test]
fn selected_clang_wrong_version_retains_unknown_cases_and_full_inventory() {
    let root = std::env::temp_dir().join(format!("cg-clang-replay-control-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("clang");
    fs::write(&tool, "#!/bin/sh\nprintf 'unknown compiler\\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let result = std::panic::catch_unwind(|| replay(tool, 90));
    fs::remove_dir_all(root).unwrap();
    let report = result.unwrap();
    assert_eq!(report["schema_version"], "0.11.0");
    assert_eq!(report["native_adapter_reused"], true);
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 12);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["native_classification"] == "unknown" && c["comparison"] == "unknown")
    );
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN; never installs"]
fn actual_clang_warnings_semantics_and_context_do_not_fabricate_syntax_labels() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").unwrap());
    let report = replay(tool, 180);
    assert_eq!(report["schema_version"], "0.11.0");
    assert_eq!(report["native_adapter_reused"], true);
    for row in report["cases"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let expected = if id.ends_with("semantic") || id.ends_with("context") {
            "unknown"
        } else if id.ends_with("syntax") {
            "invalid"
        } else {
            "valid"
        };
        assert_eq!(row["native_classification"], expected, "{row}");
        assert_eq!(
            row["native_standard"],
            if row["language"] == "c" {
                "c11"
            } else {
                "c++17"
            }
        );
        assert_eq!(
            row["fixture_native_disagreement"],
            if expected == "unknown" {
                Value::Null
            } else {
                json!(false)
            }
        );
    }
    if let Some(path) = std::env::var_os("CODEGUARD_CLANG_REPLAY_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
