#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;

#[test]
fn controlled_rust_replay_rejects_unexpected_native_output_and_preserves_inventory() {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use std::{
        collections::BTreeMap,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-rust-native-control-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("rustfmt");
    let script = "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\n[ \"$*\" = '--config-path fixed.toml --emit stdout --color never' ] || exit 2\n[ -z \"${RUSTUP_TOOLCHAIN+x}\" ] || exit 2\ninput=$(/bin/cat)\ncase \"$input\" in *'let x = ;'*) printf 'error: expected expression\\n --> <stdin>:1:1\\n' >&2; exit 1;; esac\nprintf '%s\\n' \"$input\"\n";
    fs::write(&tool, script).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    corpus["cases"].as_array_mut().unwrap().retain(|row| {
        row["language"] != "rust"
            || row["id"] == "rust-function"
            || row["id"] == "rust-missing_expr"
    });
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let tools = BTreeMap::from([("rust".into(), tool.clone())]);
    let replay = |deadline, cancelled| {
        replay_native_corpus(
            &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
            &bytes,
            &tools,
            deadline,
            &AtomicBool::new(cancelled),
        )
        .expect("explicit Rust formatter parser must be available")
    };
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert_eq!(report["schema_version"], "0.8.0");
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 2);
    assert_eq!(report["grammar_qualified_count"], 0);
    for row in report["cases"].as_array().unwrap() {
        assert_eq!(row["native"]["edition"], "2024");
        assert_eq!(row["native_identity_current"], true);
        assert_eq!(row["fixture_native_disagreement"], false, "{row}");
        assert_eq!(row["comparison"], row["combined_candidate_comparison"]);
    }
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf unexpected\nexit 0\n").unwrap();
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_classification"] == "unknown" && row["comparison"] == "unknown")
    );
    // 私有配置在版本调用中被替换时，必须在解析调用之前停止。
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'edition=\"2015\"\\n' > fixed.toml; printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\nprintf x >> \"$0.called\"\n/bin/cat\n").unwrap();
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(
                |row| row["native"]["reason"] == "rustfmt_private_config_changed"
                    && row["comparison"] == "unknown"
            )
    );
    assert!(!tool.with_extension("called").exists());
    for (deadline, cancelled) in [
        (Instant::now(), false),
        (Instant::now() + Duration::from_secs(60), true),
    ] {
        let report = replay(deadline, cancelled);
        assert_eq!(report["sample_count"], 2);
        assert!(
            report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["native_attempted"] == false && row["comparison"] == "unknown")
        );
    }
    // 请求别名变化必须在第二次调用前拒绝，批次末撤回不能代替停止执行。
    let alias = tool.with_extension("alias");
    let other = tool.with_extension("other");
    fs::write(&other, "#!/bin/sh\nprintf x >> \"$0.called\"\nexit 0\n").unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then /bin/ln -sf \"$0.other\" \"$0.alias\"; printf 'rustfmt 1.9.0-stable (fixture)\\n'; exit 0; fi\nprintf x >> \"$0.called\"\n/bin/cat >/dev/null\nexit 0\n").unwrap();
    std::os::unix::fs::symlink(&tool, &alias).unwrap();
    let redirected = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("rust".into(), alias)]),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(
        !tool.with_extension("called").exists(),
        "original canonical tool executed after request alias changed"
    );
    assert!(!root.join("rustfmt.other.called").exists());
    assert!(
        redirected["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["comparison"] == "unknown")
    );
    fs::remove_dir_all(root).unwrap();
}

fn current_corpus_bytes() -> Vec<u8> {
    use codeguard_cli::grammar_evaluation::{validate_corpus, validate_corpus_against_manifest};
    use sha2::{Digest, Sha256};
    let original = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    validate_corpus_against_manifest(
        original,
        include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_04.json"),
    )
    .unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(original).unwrap();
    corpus["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    let bytes = serde_json::to_vec(&corpus).unwrap();
    validate_corpus(&bytes).unwrap();
    bytes
}

#[test]
#[ignore = "requires explicit installed rustfmt 1.9.0-stable; set CODEGUARD_RUSTFMT_BIN"]
fn real_rustfmt_replay_parses_frozen_stdin_without_project_config_or_external_modules() {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let tool =
        PathBuf::from(std::env::var("CODEGUARD_RUSTFMT_BIN").expect("explicit native rustfmt"));
    assert!(tool.is_absolute());
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-rustfmt-isolation-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("rustfmt.toml"), "disable_all_formatting=true\n").unwrap();
    let poison = root.join("poison.rs");
    fs::write(&poison, "THIS_IS_NOT_RUST !!!\n").unwrap();
    let module = format!(
        "#[path=\"{}\"] mod external;\nfn main() {{}}\n",
        poison.display()
    );
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    for (name, source) in [
        ("unformatted", "fn HOST_SECRET(){let x=1;}"),
        ("external_module", module.as_str()),
    ] {
        corpus["cases"].as_array_mut().unwrap().push(serde_json::json!({
            "id":format!("rust-native-{name}"),"language":"rust","cohort":"provisional_syntax","label":"pending",
            "expected_valid":true,"origin":"crates/codeguard-cli/tests/rust_formatter_differential.rs",
            "source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes()))
        }));
    }
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &serde_json::to_vec(&corpus).unwrap(),
        &BTreeMap::from([("rust".into(), tool)]),
        Instant::now() + Duration::from_secs(90),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["schema_version"], "0.8.0");
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_identity_current"] == true
                && row["fixture_native_disagreement"] == false),
        "{report}"
    );
    assert!(!report.to_string().contains("HOST_SECRET"));
    assert_eq!(
        fs::read_to_string(poison).unwrap(),
        "THIS_IS_NOT_RUST !!!\n"
    );
    fs::remove_dir_all(&root).unwrap();
    if let Ok(path) = std::env::var("CODEGUARD_RUSTFMT_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
