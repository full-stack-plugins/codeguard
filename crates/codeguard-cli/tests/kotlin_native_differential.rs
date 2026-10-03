#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("empty_class", "class C {}\n", true),
        ("primary_constructor", "class C(val value: Int)\n", true),
        ("expression_function", "fun f(x: Int): Int = x + 1\n", true),
        ("data_class", "data class C(val name: String)\n", true),
        ("enumeration", "enum class C { A, B }\n", true),
        ("object", "object C { val value = 1 }\n", true),
        ("nullable", "fun f(): Int? = null\n", true),
        (
            "lambda",
            "fun f() { val g: (Int) -> Int = { it + 1 } }\n",
            true,
        ),
        ("missing_class_brace", "class C {\n", false),
        (
            "missing_function_brace",
            "fun f(x: Int) { val a = 1\n",
            false,
        ),
        ("missing_parameter_type", "fun f(x: ) = x\n", false),
        ("missing_expression", "val x =\n", false),
        ("unclosed_string", "val x = \"oops\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.kt"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "kotlin"])
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
fn kotlin_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-kotlin-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut mismatches = Vec::new();
    for (name, source, expected_valid) in corpus() {
        if candidate_is_valid(&root, name, source) != expected_valid {
            mismatches.push(name);
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(mismatches, ["missing_parameter_type"]);
}

#[test]
#[ignore = "requires explicit existing kotlinc 2.4.10 via CODEGUARD_KOTLINC_BIN"]
fn pinned_kotlin_worker_matches_native_kotlinc_on_syntax_corpus() {
    let kotlinc =
        std::env::var("CODEGUARD_KOTLINC_BIN").expect("provide an existing kotlinc executable");
    let version = Command::new(&kotlinc).arg("-version").output().unwrap();
    assert!(version.status.success());
    let version_text = format!(
        "{}{}",
        String::from_utf8_lossy(&version.stdout),
        String::from_utf8_lossy(&version.stderr)
    );
    assert!(
        version_text.contains("kotlinc-jvm 2.4.10"),
        "unexpected kotlinc version: {version_text}"
    );
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-kotlin-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let mut mismatches = Vec::new();
    for (name, source, expected_valid) in corpus() {
        let file = root.join(format!("{name}.kt"));
        fs::write(&file, source).unwrap();
        let output_dir = root.join(format!("out-{name}"));
        fs::create_dir_all(&output_dir).unwrap();
        let native = Command::new(&kotlinc)
            .arg(&file)
            .args(["-nowarn", "-d"])
            .arg(&output_dir)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: kotlinc disagrees with corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
        if candidate_is_valid(&root, name, source) != native.status.success() {
            mismatches.push(name);
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(mismatches, ["missing_parameter_type"]);
}
