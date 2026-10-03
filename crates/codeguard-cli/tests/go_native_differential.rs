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
