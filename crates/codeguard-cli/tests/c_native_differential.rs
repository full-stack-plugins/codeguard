#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

fn corpus() -> [(&'static str, &'static str, bool); 19] {
    [
        ("function", "int main(void) { return 0; }\n", true),
        ("pointer", "int f(const int *x) { return *x; }\n", true),
        ("structure", "struct Point { int x; int y; };\n", true),
        (
            "typedef",
            "typedef unsigned long Count; Count n = 0;\n",
            true,
        ),
        ("enumeration", "enum Color { RED, BLUE };\n", true),
        (
            "compound_literal",
            "struct P { int x; }; struct P p = (struct P){1};\n",
            true,
        ),
        (
            "designated",
            "struct P { int x; }; struct P p = {.x = 1};\n",
            true,
        ),
        (
            "preprocessor",
            "#define VALUE 1\nint f(void) { return VALUE; }\n",
            true,
        ),
        ("static_assert", "_Static_assert(sizeof(int) > 0, \"int size\");\n" , true),
        ("generic_selection", "int f(void) { return _Generic(1, int: 2, default: 0); }\n", true),
        ("variable_length_array", "int f(int n) { int values[n]; return sizeof(values) > 0; }\n", true),
        ("function_pointer", "int (*handler)(int);\n", true),
        ("broken_static_assert", "_Static_assert(, \"broken\");\n", false),
        ("broken_generic", "int f(void) { return _Generic(1, int: ); }\n", false),
        ("missing_brace", "int main(void) { return 0;\n", false),
        ("missing_semicolon", "int x = 1\n", false),
        (
            "missing_expr",
            "int f(void) { int x = ; return x; }\n",
            false,
        ),
        ("unclosed_string", "char *s = \"oops;\n", false),
        ("bad_declarator", "int f( { return 1; }\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.c"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "c"])
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
fn c_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-c-corpus-{}", std::process::id()));
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
#[ignore = "requires explicit existing Apple clang 21 via CODEGUARD_CLANG_BIN"]
fn pinned_c_worker_matches_native_clang_on_syntax_corpus() {
    let clang = std::env::var("CODEGUARD_CLANG_BIN").expect("provide an existing clang executable");
    let version = Command::new(&clang).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout)
            .starts_with("Apple clang version 21.0.0 (clang-2100.3.34.2)"),
        "unexpected clang version: {}",
        String::from_utf8_lossy(&version.stdout)
    );
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-c-differential-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut cases = Vec::new();
    for (name, source, expected_valid) in corpus() {
        let file = root.join(format!("{name}.c"));
        fs::write(&file, source).unwrap();
        let native = Command::new(&clang)
            .args(["-fsyntax-only", "-std=c11", "-Wno-everything", "-x", "c"])
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        if !native.status.success() {
            assert!(
                String::from_utf8_lossy(&native.stderr).contains("error:"),
                "{name}: {native:?}"
            );
        }
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: clang disagrees with corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            source,
            "{name}: native mutated source"
        );
        let candidate_valid = candidate_is_valid(&root, name, source);
        assert_eq!(
            candidate_valid,
            native.status.success(),
            "{name}: clang={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
        cases.push(serde_json::json!({"case":name,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"expected_valid":expected_valid,"native_exit":native.status.code(),"candidate_valid":candidate_valid}));
    }
    if let Ok(output) = std::env::var("CODEGUARD_C_DIFFERENTIAL_EVIDENCE") {
        let output = Path::new(&output);
        assert!(output.is_absolute(), "evidence path must be absolute");
        let evidence = serde_json::json!({
            "evidence_kind":"development_c11_native_wasm_differential",
            "qualification":"not_granted",
            "independent_holdout":false,
            "standard":"c11",
            "compiler_version":String::from_utf8_lossy(&version.stdout),
            "compiler_sha256":format!("{:x}",Sha256::digest(fs::read(&clang).unwrap())),
            "cli_sha256":format!("{:x}",Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),
            "test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("c_native_differential.rs"))),
            "grammar_manifest_sha256":format!("{:x}",Sha256::digest(fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars/manifest.json")).unwrap())),
            "case_count":cases.len(),"cases":cases,
            "delivery_decision":"not_evaluated"
        });
        fs::write(output, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
