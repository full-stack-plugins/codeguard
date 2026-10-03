#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("function", "fn main() {}\n", true),
        ("generic", "fn id<T>(x: T) -> T { x }\n", true),
        ("async", "async fn fetch() -> u8 { 1 }\n", true),
        ("macro", "macro_rules! one { () => { 1 }; }\n", true),
        ("raw_string", "const S: &str = r#\"hello\"#;\n", true),
        (
            "lifetime",
            "fn same<'a>(x: &'a str) -> &'a str { x }\n",
            true,
        ),
        (
            "match_guard",
            "fn f(x: i32) -> i32 { match x { n if n > 0 => n, _ => 0 } }\n",
            true,
        ),
        (
            "const_generic",
            "struct Buf<const N: usize>([u8; N]);\n",
            true,
        ),
        ("missing_brace", "fn main() {\n", false),
        ("missing_paren", "fn f( { }\n", false),
        ("unclosed_string", "const S: &str = \"oops;\n", false),
        ("missing_expr", "fn f() { let x = ; }\n", false),
        ("bad_match", "fn f(x: i32) { match x { => 1 } }\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.rs"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "rust"])
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
fn rust_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-rust-corpus-{}", std::process::id()));
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
#[ignore = "requires explicit existing rustfmt 1.9.0-stable via CODEGUARD_RUSTFMT_BIN"]
fn pinned_rust_worker_matches_native_rustfmt_on_syntax_corpus() {
    let rustfmt =
        std::env::var("CODEGUARD_RUSTFMT_BIN").expect("provide an existing rustfmt executable");
    let version = Command::new(&rustfmt).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout).starts_with("rustfmt 1.9.0-stable (48a229ceae "),
        "unexpected rustfmt version: {}",
        String::from_utf8_lossy(&version.stdout)
    );
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-rust-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        let file = root.join(format!("{name}.rs"));
        fs::write(&file, source).unwrap();
        let native = Command::new(&rustfmt)
            .args(["--emit", "stdout", "--edition", "2021"])
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1 | 101)),
            "{name}: {native:?}"
        );
        if !native.status.success() {
            let diagnostic = String::from_utf8_lossy(&native.stderr);
            assert!(
                diagnostic.contains("error") && !diagnostic.contains("panicked"),
                "{name}: {native:?}"
            );
        }
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: rustfmt disagrees with corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            source,
            "{name}: native mutated source"
        );
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: rustfmt={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
    }
    fs::remove_dir_all(root).unwrap();
}
