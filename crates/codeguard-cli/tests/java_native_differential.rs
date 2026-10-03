#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("empty_class", "class C {}\n", true),
        ("field", "class C { int value = 1; }\n", true),
        (
            "method",
            "class C { int f(int x) { return x + 1; } }\n",
            true,
        ),
        (
            "generic",
            "class C<T> { T value; C(T value) { this.value = value; } }\n",
            true,
        ),
        ("interface", "interface C { void run(); }\n", true),
        ("enumeration", "enum C { FIRST, SECOND }\n", true),
        ("record", "record C(int value) {}\n", true),
        (
            "lambda",
            "class C { Runnable r = () -> { int value = 1; }; }\n",
            true,
        ),
        ("missing_brace", "class C { void f() {}\n", false),
        ("missing_semicolon", "class C { int value = 1 }\n", false),
        (
            "missing_parameter_type",
            "class C { void f(value) {} }\n",
            false,
        ),
        ("missing_expression", "class C { int value = ; }\n", false),
        (
            "unclosed_string",
            "class C { String value = \"oops; }\n",
            false,
        ),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.java"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "java"])
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
fn java_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-java-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        assert_eq!(
            candidate_is_valid(&root, name, source),
            expected_valid,
            "{name}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing javac 21 via CODEGUARD_JAVAC_BIN"]
fn pinned_java_worker_matches_native_javac_on_syntax_corpus() {
    let javac = std::env::var("CODEGUARD_JAVAC_BIN").expect("provide an existing javac executable");
    let version = Command::new(&javac).arg("-version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout).starts_with("javac 21."),
        "unexpected javac version: {}",
        String::from_utf8_lossy(&version.stdout)
    );
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-java-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        let file = root.join(format!("{name}.java"));
        fs::write(&file, source).unwrap();
        let output_dir = root.join(format!("out-{name}"));
        fs::create_dir_all(&output_dir).unwrap();
        let native = Command::new(&javac)
            .args(["--release", "17", "-proc:none", "-Xlint:none", "-d"])
            .arg(&output_dir)
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: javac disagrees with corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: javac={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
    }
    fs::remove_dir_all(root).unwrap();
}
