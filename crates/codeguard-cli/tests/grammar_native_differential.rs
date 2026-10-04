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
    let corpus = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    for tools in [
        BTreeMap::new(),
        BTreeMap::from([("vbnet".into(), PathBuf::from("/not-a-tool"))]),
        BTreeMap::from([("zig".into(), PathBuf::from("relative-tool"))]),
    ] {
        assert!(
            replay_native_corpus(
                &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
                corpus,
                &tools,
                Instant::now(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
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
    let mut corpus: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
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
    assert_eq!(report["cases"][0]["comparison"], "true_negative");
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
