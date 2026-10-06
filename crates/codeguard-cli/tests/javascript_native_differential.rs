#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn controlled_javascript_replay_rejects_unexpected_native_output_and_preserves_inventory() {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use std::{
        collections::BTreeMap,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-javascript-native-control-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let tool = root.join("node");
    let script = "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'v24.18.0\\n'; exit 0; fi\n[ \"$*\" = '--check --input-type=module' ] || exit 2\n[ -z \"${NODE_OPTIONS+x}\" ] || exit 2\n[ \"$PWD\" = / ] || exit 2\ninput=$(/bin/cat)\ncase \"$input\" in 'const x = 1;'*) exit 0;; esac\nprintf '[stdin]:1\\n%s\\n^\\n\\nSyntaxError: Unexpected token\\n    at checkSyntax (node:internal/main/check_syntax:72:5)\\n\\nNode.js v24.18.0\\n' \"$input\" >&2\nexit 1\n";
    fs::write(&tool, script).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    corpus["cases"].as_array_mut().unwrap().retain(|row| {
        row["language"] != "javascript"
            || row["id"] == "javascript-plain"
            || row["id"] == "javascript-missing_init"
    });
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let tools = BTreeMap::from([("javascript".into(), tool.clone())]);
    let replay = |deadline, cancelled| {
        replay_native_corpus(
            &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
            &bytes,
            &tools,
            deadline,
            &AtomicBool::new(cancelled),
        )
        .expect("explicit Node syntax observer must be available")
    };
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert_eq!(report["schema_version"], "0.9.0");
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["sample_count"], 2);
    assert_eq!(report["grammar_qualified_count"], 0);
    for row in report["cases"].as_array().unwrap() {
        assert_eq!(row["native"]["input_type"], "module");
        assert_eq!(row["native_identity_current"], true);
        assert_eq!(row["fixture_native_disagreement"], false, "{row}");
        assert_eq!(row["comparison"], row["combined_candidate_comparison"]);
    }
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'v24.18.0\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf unexpected\nexit 0\n").unwrap();
    let report = replay(Instant::now() + Duration::from_secs(60), false);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_classification"] == "unknown" && row["comparison"] == "unknown")
    );
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
    fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = --version ]; then /bin/ln -sf \"$0.other\" \"$0.alias\"; printf 'v24.18.0\\n'; exit 0; fi\nprintf x >> \"$0.called\"\n/bin/cat >/dev/null\nexit 0\n").unwrap();
    std::os::unix::fs::symlink(&tool, &alias).unwrap();
    let redirected = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("javascript".into(), alias)]),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(
        !tool.with_extension("called").exists(),
        "original canonical tool executed after request alias changed"
    );
    assert!(!root.join("node.other.called").exists());
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

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("plain", "const x = 1;\n", true),
        ("arrow", "const f = (x) => x + 1;\n", true),
        ("optional", "const x = value?.child ?? 0;\n", true),
        (
            "class",
            "class Box { #value = 1; get value() { return this.#value; } }\n",
            true,
        ),
        (
            "import",
            "import { readFile } from \"node:fs/promises\";\n",
            true,
        ),
        ("await", "await Promise.resolve(1);\n", true),
        (
            "async",
            "async function f() { return await Promise.resolve(1); }\n",
            true,
        ),
        ("unicode", "const 名称 = \"你好\";\n", true),
        ("missing_brace", "function f() {\n", false),
        ("missing_paren", "function f( { return 1; }\n", false),
        ("unterminated", "const s = \"oops;\n", false),
        ("missing_init", "const x = ;\n", false),
        ("bad_optional", "const x = value?.;\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.js"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "javascript"])
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
fn javascript_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-javascript-corpus-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        assert_eq!(
            candidate_is_valid(&root, name, source),
            expected_valid,
            "{name}: pinned Node 24.18.0 syntax classification changed"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing Node 24.18.0 via CODEGUARD_NODE_BIN"]
fn pinned_javascript_worker_matches_native_node_check_on_syntax_corpus() {
    use codeguard_cli::grammar_native_differential::replay_native_corpus;
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let node = PathBuf::from(
        std::env::var("CODEGUARD_NODE_BIN").expect("provide an existing Node executable"),
    );
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    let extra: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/javascript_module_syntax_regression.json"
    ))
    .unwrap();
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .extend(extra["cases"].as_array().unwrap().iter().cloned());
    let bytes = serde_json::to_vec(&corpus).unwrap();
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("javascript".into(), node)]),
        Instant::now() + Duration::from_secs(300),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["schema_version"], "0.9.0");
    assert_eq!(report["sample_count"], 18);
    assert_eq!(report["program_stable"], true);
    assert_eq!(report["grammar_qualified_count"], 0);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    for row in report["cases"].as_array().unwrap() {
        assert_eq!(row["native_identity_current"], true, "{row}");
        assert_eq!(row["native"]["input_type"], "module");
        assert_eq!(row["fixture_native_disagreement"], false, "{row}");
        assert_ne!(row["comparison"], "unknown", "{row}");
        if row["id"] == "javascript-duplicate_binding" {
            assert_eq!(row["comparison"], "false_negative");
            assert_eq!(row["combined_candidate_comparison"], "true_positive");
            assert_eq!(row["structural_observations"].as_array().unwrap().len(), 1);
        } else {
            assert_eq!(row["comparison"], row["combined_candidate_comparison"]);
        }
    }
    for (name, source, expected) in self::corpus() {
        let row = report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == format!("javascript-{name}"))
            .unwrap();
        assert_eq!(
            row["source_sha256"],
            format!("{:x}", sha2::Sha256::digest(source.as_bytes()))
        );
        assert_eq!(
            row["native_classification"],
            if expected { "valid" } else { "invalid" }
        );
    }
    use sha2::Digest;
    // 每轮写独立目录，不覆盖仓库内历史原生观察。
    let root = std::env::var_os("CODEGUARD_NATIVE_DIFFERENTIAL_REPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("cg-native-javascript-{}", std::process::id()))
        });
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("javascript-native-grammar-input-cancellation-2026-10-05.json"),
        &bytes,
    )
    .unwrap();
    fs::write(
        root.join("javascript-native-grammar-differential-cancellation-2026-10-05.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
}

#[test]
fn combined_native_replay_uses_project_duplicate_binding_rule_without_rewriting_raw_result() {
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
        .join(format!("cg-js-combined-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let tool = root.join("node");
    fs::write(&tool, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'v24.18.0\\n'; exit 0; fi\n/bin/cat >/dev/null\nprintf '[stdin]:1\\nconst x=1; const x=2;\\n                 ^\\n\\nSyntaxError: Identifier x has already been declared\\n    at checkSyntax (node:internal/main/check_syntax:72:5)\\n\\nNode.js v24.18.0\\n' >&2\nexit 1\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let mut corpus: serde_json::Value = serde_json::from_slice(&current_corpus_bytes()).unwrap();
    corpus["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["language"] != "javascript" || row["id"] == "javascript-plain");
    let source = "const x=1; const x=2;\n";
    let row = corpus["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "javascript-plain")
        .unwrap();
    row["id"] = "javascript-controlled_duplicate_binding".into();
    row["source"] = source.into();
    row["source_sha256"] = format!("{:x}", Sha256::digest(source.as_bytes())).into();
    row["expected_valid"] = false.into();
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &serde_json::to_vec(&corpus).unwrap(),
        &BTreeMap::from([("javascript".into(), tool)]),
        Instant::now() + Duration::from_secs(30),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["cases"][0]["native_classification"], "invalid");
    assert_eq!(report["cases"][0]["comparison"], "false_negative");
    assert_eq!(
        report["cases"][0]["combined_candidate_comparison"], "true_positive",
        "{report}"
    );
    assert_eq!(
        report["cases"][0]["structural_observations"][0]["rule_id"],
        "codeguard.javascript.duplicate_direct_lexical_binding"
    );
    assert_eq!(report["grammar_qualified_count"], 0);
    fs::remove_dir_all(root).unwrap();
}
