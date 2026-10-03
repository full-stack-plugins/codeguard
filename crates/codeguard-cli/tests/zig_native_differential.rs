#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "requires explicit existing Zig 0.16.0 via CODEGUARD_ZIG_BIN; independent native oracle"]
fn pinned_zig_worker_matches_native_ast_check_on_syntax_corpus() {
    let zig = std::env::var("CODEGUARD_ZIG_BIN").expect("provide an existing Zig executable");
    let version = Command::new(&zig).arg("version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), "0.16.0");
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-zig-differential-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut compared = 0;
    for (name, source) in [
        ("empty_struct", "const S = struct {};\n"),
        ("empty_enum", "const E = enum {};\n"),
        ("empty_union", "const U = union(enum) {};\n"),
        ("empty_opaque", "const O = opaque {};\n"),
        ("function", "fn add(a: i32, b: i32) i32 { return a + b; }\n"),
        ("unicode_string", "const label = \"你好 Zig\";\n"),
        (
            "nested_container",
            "const S = struct { const Inner = struct {}; };\n",
        ),
        ("missing_brace", "const S = struct {\n"),
        (
            "missing_paren",
            "fn add(a: i32, b: i32 i32 { return a + b; }\n",
        ),
        ("unterminated_string", "const label = \"hello;\n"),
        ("missing_initializer", "const value = ;\n"),
    ] {
        let mut native = Command::new(&zig)
            .arg("ast-check")
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
        let file = root.join(format!("{name}.zig"));
        fs::write(&file, source).unwrap();
        let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "zig"])
            .arg(&file)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(candidate.status.code(), Some(3), "{name}: {candidate:?}");
        let report: serde_json::Value = serde_json::from_slice(&candidate.stdout).unwrap();
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
        let recoveries = report["recoveries"].as_array().unwrap();
        assert_eq!(
            recoveries.is_empty(),
            native.status.success(),
            "{name}: native={} worker={report}",
            String::from_utf8_lossy(&native.stderr)
        );
        compared += 1;
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(compared, 11);
}
