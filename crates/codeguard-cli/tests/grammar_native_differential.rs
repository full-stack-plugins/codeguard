#![cfg(all(feature = "wasm-precheck", unix))]
use codeguard_cli::grammar_native_differential::{classify_native, replay_native_corpus};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf, sync::atomic::AtomicBool, time::Instant};

#[test]
fn native_incomplete_is_unknown_and_context_diagnostics_are_not_clean() {
    assert_eq!(classify_native(&json!({"status":"completed"})), Some(true));
    assert_eq!(
        classify_native(&json!({"status":"diagnostics_observed"})),
        Some(false)
    );
    assert_eq!(
        classify_native(&json!({"status":"incomplete","diagnostics":[]})),
        None
    );
    assert_eq!(classify_native(&json!({"status":"not_run"})), None);
    let mut kotlin = json!({"status":"incomplete","reason":"kotlin_project_context_unresolved","version":"kotlinc-jvm 2.4.10","tool_sha256":"a".repeat(64),"tool_identity_scope":"launcher_only","diagnostics":[{"line":1,"column_utf16":8,"column_byte":8,"rule_id":"kotlin.syntax"}],"context_diagnostics":[{"line":1,"column_utf16":1,"column_byte":1,"rule_id":"kotlin.context.MUST_BE_INITIALIZED"}]});
    assert_eq!(
        classify_native(&kotlin),
        Some(false),
        "known syntax survives context blocker"
    );
    kotlin["diagnostics"] = json!([]);
    assert_eq!(
        classify_native(&kotlin),
        None,
        "context alone cannot classify invalid syntax"
    );
}

#[test]
fn invalid_or_empty_native_selection_is_rejected_before_processes() {
    let corpus = current_corpus_bytes();
    for tools in [
        BTreeMap::new(),
        BTreeMap::from([("vbnet".into(), PathBuf::from("/not-a-tool"))]),
        BTreeMap::from([("zig".into(), PathBuf::from("relative-tool"))]),
    ] {
        assert!(
            replay_native_corpus(
                &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
                &corpus,
                &tools,
                Instant::now(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}

#[test]
fn python_syntax_replay_uses_isolated_ruff_without_other_lint_or_project_config() {
    use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-python-differential-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("ruff");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruff 0.16.8\\n'; exit 0; fi\n[ \"$*\" = 'check --no-cache --ignore-noqa --select E9 --target-version py312 --output-format json --stdin-filename /codeguard_input.py --isolated -' ] || exit 2\ninput=$(/bin/cat)\ncase \"$input\" in 'x = 1'*) printf '[]\\n'; exit 0;; esac\nprintf '[{\"code\":\"invalid-syntax\",\"message\":\"expected token\",\"filename\":\"/codeguard_input.py\",\"severity\":\"error\",\"location\":{\"row\":1,\"column\":1}}]\\n'\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &current_corpus_bytes(),
        &BTreeMap::from([("python".into(), tool.clone())]),
        Instant::now() + Duration::from_secs(30),
        &AtomicBool::new(false),
    )
    .expect("Python must have an explicit native syntax adapter");
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 2);
    assert_eq!(
        report["cases"][0]["comparison"], "true_negative",
        "{report}"
    );
    assert_eq!(report["cases"][1]["comparison"], "true_positive");
    assert_eq!(
        report["cases"][1]["native"]["diagnostics"][0]["rule_id"],
        "invalid-syntax"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit installed Ruff 0.16.8 via CODEGUARD_RUFF_SYNTAX_BIN"]
fn actual_python_syntax_corpus_keeps_non_syntax_lint_out_of_comparison() {
    use std::{fs, time::Duration};
    let tool =
        PathBuf::from(std::env::var("CODEGUARD_RUFF_SYNTAX_BIN").expect("explicit Ruff tool"));
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    let additional: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/python_syntax_regression.json"
    ))
    .unwrap();
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .extend(additional["cases"].as_array().unwrap().iter().cloned());
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &serde_json::to_vec(&corpus).unwrap(),
        &BTreeMap::from([("python".into(), tool)]),
        Instant::now() + Duration::from_secs(180),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["sample_count"], 18);
    assert_eq!(report["program_stable"], true);
    for case in report["cases"].as_array().unwrap() {
        assert_eq!(case["native_identity_current"], true, "{case}");
        assert_eq!(case["fixture_native_disagreement"], false, "{case}");
        assert_ne!(case["native_classification"], "unknown", "{case}");
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::write(
        root.join("tests/acceptance/evidence/python-native-grammar-differential-current-manifest-2026-10-05.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
}

#[test]
fn controlled_native_replay_retains_full_inventory_and_does_not_touch_workbench() {
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-native-differential-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("zig");
    fs::write(&tool,"#!/bin/sh\nprintf x >> \"$0.calls\"\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\ninput=$(/bin/cat)\ncase \"$input\" in *'const Empty'*) exit 0;; esac\nprintf '<stdin>:1:10: error: expected token\\n' >&2\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["language"] != "zig");
    for (id, source, valid) in [
        ("zig-valid-control", "const Empty = struct {};\n", true),
        ("zig-invalid-control", "const Bad = struct {\n", false),
    ] {
        corpus["cases"].as_array_mut().unwrap().push(json!({"id":id,"language":"zig","source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"expected_valid":valid,"label":"regression","cohort":"repository_regression","origin":"tests/native_differential_control"}));
    }
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let tools = BTreeMap::from([("zig".into(), tool.clone())]);
    let replay = |deadline, cancelled| {
        replay_native_corpus(
            &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
            &bytes,
            &tools,
            deadline,
            &AtomicBool::new(cancelled),
        )
        .unwrap()
    };
    let report = replay(Instant::now() + Duration::from_secs(30), false);
    assert_eq!(report["sample_count"], 2);
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["selected_language_count"], 1);
    assert_eq!(report["independent_holdout"], false);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert_eq!(
        report["cases"][0]["comparison"], "true_negative",
        "{report}"
    );
    assert_eq!(report["cases"][1]["comparison"], "true_positive");
    assert!(!root.join(".codeguard").exists());
    let calls = fs::read(tool.with_extension("calls")).unwrap();
    for (deadline, cancelled) in [
        (Instant::now(), false),
        (Instant::now() + Duration::from_secs(30), true),
    ] {
        let report = replay(deadline, cancelled);
        assert_eq!(report["sample_count"], 2);
        assert!(
            report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["comparison"] == "unknown" && c["native_attempted"] == false)
        );
        assert_eq!(fs::read(tool.with_extension("calls")).unwrap(), calls);
    }
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '#!/bin/sh\\nprintf x >> \"$0.replaced\"\\nexit 0\\n' > \"$0\"\nexit 0\n").unwrap();
    let changed = replay(Instant::now() + Duration::from_secs(30), false);
    assert!(
        !tool.with_extension("replaced").exists(),
        "changed native tool executed again"
    );
    assert!(
        changed["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|case| case["native_classification"] == "unknown")
    );
    fs::remove_dir_all(root).unwrap();
}

// 原生差分同样必须先验证历史，再显式生成当前身份，不能绕过生产清单检查。
fn current_corpus_bytes() -> Vec<u8> {
    use codeguard_cli::grammar_evaluation::{validate_corpus, validate_corpus_against_manifest};
    use sha2::{Digest, Sha256};
    let archived = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    validate_corpus_against_manifest(
        archived,
        include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_04.json"),
    )
    .unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(archived).unwrap();
    let cases = corpus["cases"].clone();
    corpus["manifest_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    assert_eq!(corpus["cases"], cases);
    let bytes = serde_json::to_vec(&corpus).unwrap();
    validate_corpus(&bytes).unwrap();
    bytes
}

#[test]
fn historical_native_corpus_is_rejected_before_tool_execution() {
    let result = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json"),
        &BTreeMap::from([("python".into(), PathBuf::from("/does-not-exist/ruff"))]),
        Instant::now(),
        &AtomicBool::new(false),
    );
    assert_eq!(
        result.unwrap_err(),
        "grammar_evaluation_corpus_identity_invalid"
    );
}
