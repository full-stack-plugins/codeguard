#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::process::Command;

#[test]
fn public_candidate_probe_executes_pinned_worker_without_claiming_lint() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-grammar-probe-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (language, filename, source, expect_recoveries) in [
        ("python", "a.py", "x = 1\n", false),
        ("tsx", "a.tsx", "const C = () => <div />;\n", false),
        ("java", "A.java", "class A {", true),
    ] {
        let path = root.join(filename);
        fs::write(&path, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", language])
            .arg(&path)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{language}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["report_type"], "grammar_candidate_probe");
        assert_eq!(report["precheck"]["status"], "incomplete");
        assert_eq!(report["native"]["status"], "not_run");
        assert_eq!(report["delivery_decision"], "not_evaluated");
        assert_eq!(
            report["recoveries"].as_array().unwrap().is_empty(),
            !expect_recoveries,
            "{language}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}
