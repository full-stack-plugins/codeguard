#![cfg(all(feature = "wasm-precheck", unix))]

use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

/// 使用既有工具比较 C++17 语法，类型语义单独保留。
#[test]
#[ignore = "requires explicit existing Apple clang21 via CODEGUARD_CLANG_BIN"]
fn cpp17_native_and_wasm_syntax_boundaries() {
    let clang = std::env::var("CODEGUARD_CLANG_BIN").expect("explicit existing compiler");
    let version = Command::new(&clang).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout)
            .starts_with("Apple clang version 21.0.0 (clang-2100.3.34.2)")
    );
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-cpp17-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let cases = [
        (
            "namespace",
            "namespace n { int f() { return 1; } }",
            true,
            true,
        ),
        (
            "template",
            "template<class T> T identity(T x) { return x; }",
            true,
            true,
        ),
        (
            "lambda",
            "int f() { auto g = [](int x) { return x + 1; }; return g(1); }",
            true,
            true,
        ),
        (
            "structured_binding",
            "struct P { int x; int y; }; int f() { auto [x,y] = P{1,2}; return x+y; }",
            true,
            true,
        ),
        (
            "constexpr_if",
            "template<class T> int f(T x) { if constexpr(sizeof(T)>1) return 1; else return 0; }",
            true,
            true,
        ),
        (
            "fold_expression",
            "template<class... T> auto sum(T... x) { return (x + ...); }",
            true,
            true,
        ),
        (
            "raw_string",
            "const char *s = R\"tag(a\"b)tag\";",
            true,
            true,
        ),
        ("missing_brace", "int f() { return 0;", false, false),
        ("missing_expression", "int f() { return ; + }", false, false),
        ("missing_semicolon", "int value = 1", false, false),
        ("type_mismatch", "int value = \"text\";", false, true),
        ("unknown_symbol", "int f() { return absent; }", false, true),
    ];
    let mut observations = Vec::new();
    for (name, source, native_valid, wasm_valid) in cases {
        let file = root.join(format!("{name}.cpp"));
        fs::write(&file, source).unwrap();
        let native = Command::new(&clang)
            .args([
                "-fsyntax-only",
                "-std=c++17",
                "-Wno-everything",
                "-x",
                "c++",
            ])
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            native_valid,
            "{name}: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "cpp"])
            .arg(&file)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(candidate.status.code(), Some(3), "{name}: {candidate:?}");
        let report: serde_json::Value = serde_json::from_slice(&candidate.stdout).unwrap();
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert!(report["reason"].is_null(), "{name}: {report}");
        assert_eq!(
            report["recoveries"].as_array().unwrap().is_empty(),
            wasm_valid,
            "{name}: {report}"
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
        observations.push(serde_json::json!({
            "name": name,
            "source_sha256": format!("{:x}", Sha256::digest(source.as_bytes())),
            "native_exit_code": native.status.code(),
            "native_valid": native.status.success(),
            "wasm_valid": report["recoveries"].as_array().unwrap().is_empty(),
            "expected_native_valid": native_valid,
            "expected_wasm_valid": wasm_valid,
            "grammar_qualified": report["grammar_qualified"],
            "delivery_decision": report["delivery_decision"]
        }));
    }
    fs::remove_dir_all(root).unwrap();
    if let Ok(destination) = std::env::var("CODEGUARD_CPP_DIFFERENTIAL_EVIDENCE") {
        let path = std::path::Path::new(&destination);
        assert!(path.is_absolute(), "evidence destination must be absolute");
        let evidence = serde_json::json!({
            "schema_version": "0.1",
            "language": "cpp",
            "standard": "c++17",
            "qualification": "not_granted",
            "corpus_kind": "author_regression_not_independent_holdout",
            "compiler_version": String::from_utf8_lossy(&version.stdout).trim(),
            "compiler_sha256": format!("{:x}", Sha256::digest(fs::read(&clang).unwrap())),
            "cli_sha256": format!("{:x}", Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())),
            "test_source_sha256": format!("{:x}", Sha256::digest(include_bytes!("cpp_native_differential.rs"))),
            "cases": observations
        });
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
