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

#[test]
fn missing_source_has_versioned_incomplete_json_and_closed_schema() {
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/grammar-probe-v0.1.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).expect("public probe schema")).unwrap();
    assert_eq!(schema["$id"], "urn:codeguard:schema:grammar-probe:0.1.0");
    assert_eq!(schema["additionalProperties"], false);

    let path = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-missing-probe-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "zig"])
        .arg(&path)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.1.0");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["native"]["status"], "not_run");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["reason"].as_str().unwrap().contains("source"));

    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars/manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut manifest_languages = manifest["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| asset["language"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut schema_languages = schema["properties"]["language"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|language| language.as_str().unwrap())
        .collect::<Vec<_>>();
    manifest_languages.sort_unstable();
    schema_languages.sort_unstable();
    assert_eq!(schema_languages, manifest_languages);
}

#[test]
fn hidden_parser_errors_require_native_confirmation_without_invented_positions() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-hidden-probe-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (language, source) in [
        ("swift", "func f(_ x: ) {}\n"),
        ("kotlin", "fun f(x: ) = x\n"),
        ("kotlin", "object C { val value = 1 }\n"),
    ] {
        let path = root.join("sample.txt");
        fs::write(&path, source).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", language])
            .arg(&path)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(3));
        let report: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(report["schema_version"], "0.5.0", "{report}");
        assert_eq!(report["parser_error_location_unavailable"], true);
        assert_eq!(
            report["next_action"],
            "compare_original_source_with_native_tool_then_review_grammar"
        );
        assert!(report["recoveries"].as_array().unwrap().is_empty());
        assert_eq!(report["precheck"]["truncated_files"], 1);
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
    fs::remove_dir_all(root).unwrap();
}
