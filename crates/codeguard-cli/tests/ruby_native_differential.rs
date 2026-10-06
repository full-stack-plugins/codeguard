#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("plain", "x = 1\n", true),
        ("class", "class Box\n  def value\n    1\n  end\nend\n", true),
        ("block", "[1, 2].map { |x| x + 1 }\n", true),
        ("hash", "h = { name: \"a\", count: 2 }\n", true),
        (
            "interpolation",
            "name = \"world\"\ns = \"hello #{name}\"\n",
            true,
        ),
        ("heredoc", "s = <<~TEXT\n  hello\nTEXT\n", true),
        ("modifier", "puts \"ok\" if true\n", true),
        ("module", "module M\n  CONST = 1\nend\n", true),
        (
            "missing_end",
            "class Box\n  def value\n    1\n  end\n",
            false,
        ),
        ("unclosed_string", "s = \"oops\n", false),
        ("bad_def", "def f(\n  1\nend\n", false),
        ("missing_expr", "x =\n", false),
        ("bad_call", "foo(1, 2\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.rb"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "ruby"])
        .arg(&file)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(candidate.status.code(), Some(3), "{name}: {candidate:?}");
    let report: serde_json::Value = serde_json::from_slice(&candidate.stdout).unwrap();
    assert_eq!(report["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["reason"],
        serde_json::Value::Null,
        "{name}: {report}"
    );
    report["recoveries"].as_array().unwrap().is_empty()
}

#[test]
fn ruby_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruby-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        assert_eq!(
            candidate_is_valid(&root, name, source),
            expected_valid,
            "{name}: pinned Ruby 2.6.10 syntax classification changed"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing Ruby 2.6.10 via CODEGUARD_RUBY_BIN"]
fn pinned_ruby_worker_matches_native_ruby_check_on_syntax_corpus() {
    let ruby = std::env::var("CODEGUARD_RUBY_BIN").expect("provide an existing Ruby executable");
    let version = Command::new(&ruby).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("ruby 2.6.10p210 "));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-ruby-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let mut compared = 0;
    for (name, source, expected_valid) in corpus() {
        let mut native = Command::new(&ruby)
            .args(["-c", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        native
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let native = native.wait_with_output().unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: Ruby disagrees with the pinned corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: Ruby={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
        compared += 1;
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(compared, 13);
}

#[test]
fn controlled_ruby_replay_uses_explicit_isolated_native_observer() {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use sha2::{Digest, Sha256};
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
        .join(format!("cg-ruby-native-control-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let tool = root.join("ruby");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'ruby 2.6.10p210 (fixture) [fixture]\\n'; exit 0; fi\n[ \"$*\" = '--disable=gems -EUTF-8:UTF-8 -W0 -c -' ] || exit 2\n[ -z \"${RUBYOPT+x}\" ] || exit 2\n[ \"$PWD\" = / ] || exit 2\n/bin/cat >/dev/null\nprintf 'Syntax OK\\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    corpus["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("ruby".into(), tool)]),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(report["schema_version"], "0.5.0");
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 14);
    assert_eq!(report["native_adapter_reused"], false);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_classification"] == "valid")
    );
    assert_eq!(
        report["languages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["native_selected"] == true)
            .count(),
        1
    );
}

#[test]
#[ignore = "requires explicit existing Ruby 2.6.10p210 via CODEGUARD_RUBY_BIN"]
fn pinned_ruby_uniform_replay_archives_frozen_native_and_wasm_evidence() {
    use codeguard_cli::{
        grammar_evaluation::{validate_corpus, validate_corpus_against_manifest},
        grammar_native_differential::replay_native_corpus,
    };
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let original = include_bytes!("../../../tests/fixtures/grammar_regression_v0_2.json");
    validate_corpus_against_manifest(
        original,
        include_bytes!("../../../tests/fixtures/grammar_manifests/manifest_2026_10_04.json"),
    )
    .unwrap();
    let mut input: serde_json::Value = serde_json::from_slice(original).unwrap();
    input["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    for (id, source) in [
        (
            "ruby-no_execution",
            "BEGIN { raise \"BEGIN must not execute\" }\nEND { raise \"END must not execute\" }\nraise \"body must not execute\"\n",
        ),
        (
            "ruby-no_require_resolution",
            "require \"codeguard_nonexistent_module_must_not_load\"\nx = 1\n",
        ),
        (
            "ruby-no_shebang_require",
            "#!/usr/bin/ruby -r/codeguard_nonexistent_must_not_load.rb\nx = 1\n",
        ),
        ("ruby-utf8_encoding", "# coding: utf-8\n名称 = \"你好\"\n"),
    ] {
        input["cases"].as_array_mut().unwrap().push(serde_json::json!({"id":id,"language":"ruby","source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"expected_valid":true,"label":"regression","cohort":"repository_regression","origin":"crates/codeguard-cli/tests/ruby_native_differential.rs"}));
    }
    let bytes = serde_json::to_vec_pretty(&input).unwrap();
    validate_corpus(&bytes).unwrap();
    let ruby = PathBuf::from(
        std::env::var("CODEGUARD_RUBY_BIN").expect("provide an existing Ruby executable"),
    );
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("ruby".into(), ruby)]),
        Instant::now() + Duration::from_secs(120),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["sample_count"], 18);
    assert_eq!(report["schema_version"], "0.5.0");
    let ruby = report["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["language"] == "ruby")
        .unwrap();
    assert_eq!(
        (
            ruby["tp"].as_u64(),
            ruby["fp"].as_u64(),
            ruby["fn"].as_u64(),
            ruby["tn"].as_u64()
        ),
        (Some(5), Some(0), Some(0), Some(13))
    );
    assert_eq!(ruby["native_unknown_count"], 0);
    assert_eq!(ruby["wasm_unknown_count"], 0);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_identity_current"] == true
                && row["fixture_native_disagreement"] == false)
    );
    // 独立输出本轮证据，保留旧源码/工具身份绑定的历史报告。
    let evidence = std::env::var_os("CODEGUARD_NATIVE_DIFFERENTIAL_REPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("cg-native-ruby-{}", std::process::id()))
        });
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("ruby-native-grammar-input-2026-10-05.json"),
        bytes,
    )
    .unwrap();
    fs::write(
        evidence.join("ruby-native-grammar-differential-2026-10-05.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
