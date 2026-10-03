#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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
    let node = std::env::var("CODEGUARD_NODE_BIN").expect("provide an existing Node executable");
    let version = Command::new(&node).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), "v24.18.0");
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-javascript-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let mut compared = 0;
    for (name, source, expected_valid) in corpus() {
        let mut native = Command::new(&node)
            .args(["--check", "--input-type=module"])
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
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: Node disagrees with the pinned corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: Node={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
        compared += 1;
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(compared, 13);
}
