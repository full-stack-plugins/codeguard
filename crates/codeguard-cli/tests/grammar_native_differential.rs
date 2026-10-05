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
    assert_eq!(report["schema_version"], "0.3.0");
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
    assert_eq!(report["schema_version"], "0.3.0");
    assert_eq!(report["sample_count"], 18);
    assert_eq!(report["program_stable"], true);
    for case in report["cases"].as_array().unwrap() {
        assert_eq!(case["native_identity_current"], true, "{case}");
        assert_eq!(case["fixture_native_disagreement"], false, "{case}");
        assert_ne!(case["native_classification"], "unknown", "{case}");
    }
    let language = report["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["language"] == "python")
        .unwrap();
    for (key, expected) in [("tp", 6), ("fp", 0), ("fn", 2), ("tn", 10)] {
        assert_eq!(language[key], expected, "raw grammar: {report}");
    }
    for (key, expected) in [
        ("tp", 8),
        ("fp", 0),
        ("fn", 0),
        ("tn", 10),
        ("compared_count", 18),
        ("unknown_count", 0),
    ] {
        assert_eq!(
            language["combined_candidate"][key], expected,
            "combined candidate: {report}"
        );
    }
    for id in ["python-empty_body", "python-bad_indent"] {
        let row = report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap();
        assert_eq!(row["comparison"], "false_negative");
        assert_eq!(row["wasm_recovery_count"], 0);
        assert_eq!(row["combined_candidate_comparison"], "true_positive");
        assert_eq!(
            row["structural_observations"][0]["rule_id"],
            "codeguard.python.required_suite"
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::write(
        root.join("tests/acceptance/evidence/python-native-structure-differential-2026-10-05.json"),
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

#[test]
fn python_structure_measurement_preserves_raw_false_negatives_and_unknowns() {
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-structure-differential-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("ruff");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruff 0.16.8\\n'; exit 0; fi\n[ \"$*\" = 'check --no-cache --ignore-noqa --select E9 --target-version py312 --output-format json --stdin-filename /codeguard_input.py --isolated -' ] || exit 2\ninput=$(/bin/cat)\ncase \"$input\" in *'    pass'*) printf '[]\\n'; exit 0;; esac\nprintf '[{\"code\":\"invalid-syntax\",\"message\":\"expected suite\",\"filename\":\"/codeguard_input.py\",\"severity\":\"error\",\"location\":{\"row\":1,\"column\":1}}]\\n'\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|case| case["language"] != "python");
    for (id, source, valid) in [
        ("python-empty_body", "def run():\n", false),
        ("python-bad_indent", "if True:\npass\n", false),
        ("python-valid-suite", "def run():\n    pass\n", true),
    ] {
        corpus["cases"].as_array_mut().unwrap().push(json!({"id":id,"language":"python","source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"expected_valid":valid,"label":"regression","cohort":"repository_regression","origin":"tests/structure_differential_control"}));
    }
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let tools = BTreeMap::from([("python".into(), tool.clone())]);
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
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert_eq!(
        report["cases"][0]["native_classification"], "invalid",
        "{report}"
    );
    assert_eq!(report["schema_version"], "0.3.0", "{report}");
    let rows = report["cases"].as_array().unwrap();
    for row in &rows[..2] {
        assert_eq!(row["wasm_classification"], "valid", "{row}");
        assert_eq!(row["comparison"], "false_negative");
        assert_eq!(row["combined_candidate_classification"], "invalid");
        assert_eq!(row["combined_candidate_comparison"], "true_positive");
        assert_eq!(row["structural_observations"].as_array().unwrap().len(), 1);
        assert_eq!(
            row["structural_observations"][0]["rule_id"],
            "codeguard.python.required_suite"
        );
        assert_eq!(
            row["structural_observations"][0]["rule_sha256"],
            codeguard_adapters::python_suite_rule_sha256()
        );
    }
    assert_eq!(rows[2]["structural_observations"], json!([]));
    assert_eq!(rows[2]["combined_candidate_comparison"], "true_negative");
    let language = report["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["language"] == "python")
        .unwrap();
    assert_eq!(language["fn"], 2);
    assert_eq!(language["combined_candidate"]["fn"], 0);
    assert_eq!(language["combined_candidate"]["tp"], 2);
    assert_eq!(language["combined_candidate"]["tn"], 1);
    assert_eq!(language["combined_candidate"]["compared_count"], 3);
    for (deadline, cancelled) in [
        (Instant::now(), false),
        (Instant::now() + Duration::from_secs(60), true),
    ] {
        let unknown = replay(deadline, cancelled);
        for row in unknown["cases"].as_array().unwrap() {
            assert_eq!(row["wasm_classification"], "unknown");
            assert_eq!(row["combined_candidate_classification"], "unknown");
            assert_eq!(row["combined_candidate_comparison"], "unknown");
            assert_eq!(row["structural_observations"], serde_json::Value::Null);
        }
        let language = unknown["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "python")
            .unwrap();
        assert_eq!(language["sample_count"], 3);
        assert_eq!(language["combined_candidate"]["unknown_count"], 3);
        assert_eq!(language["combined_candidate"]["compared_count"], 0);
    }
    // 原生入口变化只撤回 oracle 和两层比较，不能把真实结构观察丢成源码通过。
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruff 0.16.8\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '#!/bin/sh\\nexit 0\\n' > \"$0\"\nprintf '[]\\n'\nexit 0\n").unwrap();
    let changed_tool = replay(Instant::now() + Duration::from_secs(60), false);
    for row in changed_tool["cases"].as_array().unwrap() {
        assert_eq!(row["native_identity_current"], false);
        assert_eq!(row["comparison"], "unknown");
        assert_eq!(row["combined_candidate_comparison"], "unknown");
    }
    assert_eq!(
        changed_tool["cases"][0]["combined_candidate_classification"],
        "invalid"
    );
    // 受控 worker 首次返回合法结构证据后改写自己；批次末必须撤回两层观察。
    let worker = root.join("changing-worker");
    let asset = codeguard_adapters::bundled_grammar_candidate("python")
        .unwrap()
        .0;
    let worker_report = json!({"schema_version":"1.1.0","report_type":"syntax_worker_candidate","language":"python","grammar_sha256":asset.sha256,"grammar_abi_version":asset.abi_version,"source_sha256":rows[0]["source_sha256"],"truncated":false,"recoveries":[],"structural_observations":rows[0]["structural_observations"]});
    let mut truncated_report = worker_report.clone();
    truncated_report["truncated"] = json!(true);
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\n/bin/cat >/dev/null\nprintf '%s\\n' '{}'\n",
            truncated_report
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let truncated = replay_native_corpus(
        &worker,
        &bytes,
        &tools,
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(truncated["program_stable"], true);
    assert_eq!(
        truncated["cases"][0]["wasm_reason"],
        "syntax_recovery_incomplete"
    );
    assert_eq!(
        truncated["cases"][0]["structural_observations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(truncated["cases"][0]["wasm_classification"], "unknown");
    assert_eq!(
        truncated["cases"][0]["combined_candidate_classification"],
        "unknown"
    );
    fs::write(&worker, format!("#!/bin/sh\n/bin/cat >/dev/null\nprintf '%s\\n' '{}'\nprintf '#!/bin/sh\\nexit 1\\n' > \"$0\"\n", worker_report)).unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let changed_program = replay_native_corpus(
        &worker,
        &bytes,
        &tools,
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(changed_program["program_stable"], false);
    for row in changed_program["cases"].as_array().unwrap() {
        assert_eq!(row["wasm_reason"], "grammar_evaluation_program_changed");
        assert_eq!(row["structural_observations"], serde_json::Value::Null);
        assert_eq!(row["combined_candidate_classification"], "unknown");
        assert_eq!(row["combined_candidate_comparison"], "unknown");
    }
    fs::remove_dir_all(root).unwrap();
}
