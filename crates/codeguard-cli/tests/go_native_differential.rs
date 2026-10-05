#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("empty", "package main\nfunc main() {}\n", true),
        (
            "generics",
            "package p\nfunc Id[T any](x T) T { return x }\n",
            true,
        ),
        (
            "range_int",
            "package p\nfunc f() { for i := range 3 { _ = i } }\n",
            true,
        ),
        ("unicode", "package p\nvar 世界 = \"ok\"\n", true),
        ("raw_string", "package p\nconst s = `hello\nworld`\n", true),
        (
            "composite",
            "package p\ntype P struct { X int }\nvar p = P{X: 1}\n",
            true,
        ),
        (
            "interface",
            "package p\ntype I interface { F() error }\n",
            true,
        ),
        (
            "switch",
            "package p\nfunc f(x any) { switch x.(type) { case int: return; default: return } }\n",
            true,
        ),
        ("missing_brace", "package p\nfunc f() {\n", false),
        ("missing_paren", "package p\nfunc f( { }\n", false),
        (
            "unterminated_string",
            "package p\nconst s = \"oops\n",
            false,
        ),
        ("missing_init", "package p\nvar x =\n", false),
        ("bad_if", "package p\nfunc f() { if { } }\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.go"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "go"])
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
fn go_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-go-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        assert_eq!(
            candidate_is_valid(&root, name, source),
            expected_valid,
            "{name}: pinned Go 1.23.4 syntax classification changed"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing Go 1.23.4 via CODEGUARD_GO_BIN and CODEGUARD_GOFMT_BIN"]
fn pinned_go_worker_matches_native_gofmt_on_syntax_corpus() {
    let go = std::env::var("CODEGUARD_GO_BIN").expect("provide an existing Go executable");
    let gofmt = std::env::var("CODEGUARD_GOFMT_BIN").expect("provide matching gofmt executable");
    let version = Command::new(&go).arg("version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout).starts_with("go version go1.23.4 "),
        "unexpected Go version: {}",
        String::from_utf8_lossy(&version.stdout)
    );
    assert_eq!(
        fs::canonicalize(&go).unwrap().parent(),
        fs::canonicalize(&gofmt).unwrap().parent(),
        "Go and gofmt must come from the same toolchain directory"
    );
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-go-differential-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut compared = 0;
    for (name, source, expected_valid) in corpus() {
        let mut native = Command::new(&gofmt)
            .arg("-e")
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
            matches!(native.status.code(), Some(0 | 2)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: gofmt disagrees with the pinned corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: gofmt={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
        compared += 1;
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(compared, 13);
}

#[test]
fn controlled_go_uniform_replay_requires_whole_file_and_both_sdk_tools() {
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
        .join(format!("cg-go-uniform-control-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let go = root.join("go");
    let gofmt = root.join("gofmt");
    fs::write(&go,"#!/bin/sh\n[ \"$GOTOOLCHAIN\" = local ] || exit 2\n[ \"$GOENV\" = off ] || exit 2\n[ \"$GOPROXY\" = off ] || exit 2\n[ \"$1\" = version ] || exit 2\nif [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi\n").unwrap();
    fs::write(&gofmt,"#!/bin/sh\n[ \"$*\" = '-e /dev/stdin' ] || exit 3\n[ -z \"${GOFLAGS+x}\" ] || exit 3\n[ \"$PWD\" = / ] || exit 3\n/bin/cat\n").unwrap();
    for tool in [&go, &gofmt] {
        fs::set_permissions(tool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut corpus: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    corpus["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &serde_json::to_vec(&corpus).unwrap(),
        &BTreeMap::from([("go".into(), go)]),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(report["schema_version"], "0.7.0");
    assert_eq!(report["sample_count"], 14);
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["native_adapter_reused"], false);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_classification"] == "valid")
    );
}

#[test]
#[ignore = "requires explicit existing Go1.23.4 SDK via CODEGUARD_GO_BIN"]
fn pinned_go_uniform_replay_archives_whole_file_and_companion_evidence() {
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
    for (id, source, valid) in [
        ("go-missing_package_statement", "x := 1\n", false),
        ("go-missing_package_function", "func f() {}\n", false),
        (
            "go-no_import_resolution",
            "package p\nimport \"codeguard_nonexistent_module\"\n",
            true,
        ),
        (
            "go-no_init_execution",
            "package p\nfunc init(){panic(\"must not execute\")}\n",
            true,
        ),
        (
            "go-format_difference_not_violation",
            "package p;var x=1\n",
            true,
        ),
        (
            "go-logical_line_positions_unresolved",
            "package p\n//line /dev/stdin:1\nvar x =\n",
            false,
        ),
    ] {
        input["cases"].as_array_mut().unwrap().push(serde_json::json!({"id":id,"language":"go","source":source,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"expected_valid":valid,"label":"regression","cohort":"repository_regression","origin":"crates/codeguard-cli/tests/go_native_differential.rs"}));
    }
    let bytes = serde_json::to_vec_pretty(&input).unwrap();
    validate_corpus(&bytes).unwrap();
    let go = PathBuf::from(
        std::env::var("CODEGUARD_GO_BIN").expect("provide an existing Go SDK executable"),
    );
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &bytes,
        &BTreeMap::from([("go".into(), go)]),
        Instant::now() + Duration::from_secs(120),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["schema_version"], "0.7.0");
    assert_eq!(report["sample_count"], 20);
    assert_eq!(report["language_count"], 32);
    assert_eq!(report["grammar_qualified_count"], 0);
    let rows = report["cases"].as_array().unwrap();
    assert!(
        rows.iter()
            .all(|row| row["fixture_native_disagreement"] != true)
    );
    let unknown = rows
        .iter()
        .find(|row| row["id"] == "go-logical_line_positions_unresolved")
        .unwrap();
    assert_eq!(unknown["native_classification"], "unknown");
    assert_eq!(
        unknown["native"]["reason"],
        "go_syntax_logical_positions_unresolved"
    );
    for id in [
        "go-missing_package_statement",
        "go-missing_package_function",
    ] {
        assert_eq!(
            rows.iter().find(|row| row["id"] == id).unwrap()["native_classification"],
            "invalid"
        );
    }
    let go = report["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["language"] == "go")
        .unwrap();
    assert_eq!(
        go["native_unknown_count"], 1,
        "只有未解析逻辑位置留作原生未知"
    );
    assert_eq!(go["compared_count"], 19, "不能丢弃EOF语法诊断的样本");
    let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/evidence");
    fs::write(
        evidence.join("go-native-grammar-package-input-2026-10-05.json"),
        bytes,
    )
    .unwrap();
    fs::write(
        evidence.join("go-native-grammar-package-differential-2026-10-05.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        report["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "go")
            .unwrap()
    );
}

#[test]
fn changing_gofmt_within_batch_withdraws_earlier_comparisons() {
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
        .join(format!("cg-go-companion-batch-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let go = root.join("go");
    let fmt = root.join("gofmt");
    let other = root.join("gofmt.other");
    fs::write(&go,"#!/bin/sh\nif [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi\n").unwrap();
    fs::write(&fmt,"#!/bin/sh\nif [ -f \"$0.once\" ]; then /bin/mv \"$0.other\" \"$0\"; else : > \"$0.once\"; fi\n/bin/cat\n").unwrap();
    fs::write(&other, "#!/bin/sh\n# replacement artifact\n/bin/cat\n").unwrap();
    for tool in [&go, &fmt, &other] {
        fs::set_permissions(tool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut corpus: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    corpus["manifest_sha256"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("../../../grammars/manifest.json"))
    ));
    corpus["cases"].as_array_mut().unwrap().retain(|row| {
        row["language"] != "go"
            || ["go-empty", "go-generics"].contains(&row["id"].as_str().unwrap())
    });
    let report = replay_native_corpus(
        &PathBuf::from(env!("CARGO_BIN_EXE_codeguard")),
        &serde_json::to_vec(&corpus).unwrap(),
        &BTreeMap::from([("go".into(), go)]),
        Instant::now() + Duration::from_secs(60),
        &AtomicBool::new(false),
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(report["sample_count"], 2);
    assert!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["native_classification"] == "unknown"
                && row["comparison"] == "unknown"
                && row["combined_candidate_comparison"] == "unknown"),
        "{report}"
    );
    assert_eq!(report["cases"][0]["native"]["status"], "completed");
    assert_eq!(
        report["cases"][1]["native"]["reason"],
        "go_syntax_tool_changed"
    );
    assert_eq!(
        report["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "go")
            .unwrap()["tool_stable"],
        false
    );
}
