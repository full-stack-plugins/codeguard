#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use sha2::Digest;
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

fn candidate_report(root: &Path, name: &str, source: &str) -> serde_json::Value {
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
    report
}

fn candidate_classification(report: &serde_json::Value) -> Option<bool> {
    if report["precheck"]["truncated_files"].as_u64().unwrap() != 0 {
        None
    } else {
        Some(report["recoveries"].as_array().unwrap().is_empty())
    }
}

#[test]
fn hidden_java_recovery_is_unknown_instead_of_clean() {
    let report = serde_json::json!({"precheck":{"truncated_files":1},"recoveries":[]});
    assert_eq!(candidate_classification(&report), None);
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
            candidate_classification(&candidate_report(&root, name, source)),
            Some(expected_valid),
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
            candidate_classification(&candidate_report(&root, name, source)),
            Some(native.status.success()),
            "{name}: javac={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
    }
    fs::remove_dir_all(root).unwrap();
}

/// 本轮新增的Java21样本与既有固定回归语料分开统计，不授予grammar资格。
#[test]
#[ignore = "requires explicit existing javac 21 via CODEGUARD_JAVAC_BIN"]
fn java21_language_forms_match_existing_native_compiler() {
    let javac = std::env::var("CODEGUARD_JAVAC_BIN").unwrap();
    let tool_bytes = fs::read(&javac).unwrap();
    let tool_sha256 = format!("{:x}", sha2::Sha256::digest(&tool_bytes));
    let version = Command::new(&javac).arg("-version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("javac 21."));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-java21-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let cases = [
        (
            "pattern_switch",
            "class C { int f(Object o) { return switch(o) { case String s -> s.length(); case Integer i -> i; default -> 0; }; } }",
            true,
        ),
        (
            "guarded_switch",
            "class C { int f(Object o) { return switch(o) { case String s when s.isEmpty() -> 0; case String s -> 1; default -> 2; }; } }",
            true,
        ),
        (
            "record_pattern",
            "record P(int x, int y) {} class C { int f(Object o) { return o instanceof P(int x, int y) ? x + y : 0; } }",
            true,
        ),
        (
            "sealed_record",
            "sealed interface S permits P {} record P(int x) implements S {}",
            true,
        ),
        (
            "text_block",
            "class C { String s = \"\"\"\nhello\n\"\"\"; }",
            true,
        ),
        (
            "missing_guard",
            "class C { int f(Object o) { return switch(o) { case String s when -> 0; default -> 2; }; } }",
            false,
        ),
        (
            "missing_record_parameter_type",
            "record P(int x) {} class C { boolean f(Object o) { return o instanceof P(x); } }",
            false,
        ),
        (
            "missing_switch_arrow",
            "class C { int f(Object o) { return switch(o) { case String s s.length(); default -> 0; }; } }",
            false,
        ),
    ];
    let mut observations = Vec::new();
    for (name, source, expected_valid) in cases {
        let file = root.join(format!("{name}.java"));
        fs::write(&file, source).unwrap();
        let out = root.join(format!("out-{name}"));
        fs::create_dir_all(&out).unwrap();
        let native = Command::new(&javac)
            .args(["--release", "21", "-proc:none", "-Xlint:none", "-d"])
            .arg(&out)
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
            "{name}: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        let candidate = candidate_report(&root, name, source);
        let classification = candidate_classification(&candidate);
        observations.push(serde_json::json!({"case":name,"source_sha256":format!("{:x}",sha2::Sha256::digest(source.as_bytes())),"native_valid":native.status.success(),"candidate_valid":classification,"candidate":candidate}));
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
    }
    assert_eq!(
        fs::read(&javac).unwrap(),
        tool_bytes,
        "native compiler changed during oracle execution"
    );
    if let Ok(path) = std::env::var("CODEGUARD_JAVA21_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"schema_version":"0.1.0","scope":"java21_new_local_cases","native_tool_sha256":tool_sha256,"native_version":String::from_utf8_lossy(&version.stdout),"independent_holdout":false,"grammar_qualified":false,"delivery_decision":"not_evaluated","observations":observations})).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
    let disagreements: Vec<_> = observations
        .iter()
        .filter(|r| r["candidate_valid"].is_boolean() && r["candidate_valid"] != r["native_valid"])
        .map(|r| r["case"].clone())
        .collect();
    assert!(
        disagreements.is_empty(),
        "Java21 decidable differences: {disagreements:?}"
    );
}
