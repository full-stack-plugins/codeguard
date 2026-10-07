#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::path::Path;
use std::process::Command;

fn corpus() -> Vec<(String, String, bool)> {
    let mut cases = vec![
        ("simple", "-module(sample).\nf() -> ok.\n", true),
        ("argument", "-module(sample).\nf(X) -> X + 1.\n", true),
        (
            "guard",
            "-module(sample).\nf(X) when is_integer(X) -> X.\n",
            true,
        ),
        ("list", "-module(sample).\nf() -> [1, 2, 3].\n", true),
        ("map", "-module(sample).\nf() -> #{a => 1}.\n", true),
        (
            "case",
            "-module(sample).\nf(X) -> case X of 1 -> one; _ -> other end.\n",
            true,
        ),
        (
            "list_comprehension",
            "-module(sample).\nf() -> [X * 2 || X <- [1, 2]].\n",
            true,
        ),
        (
            "record",
            "-module(sample).\n-record(user, {id}).\nf() -> #user{id = 1}.\n",
            true,
        ),
        ("missing_period", "-module(sample).\nf() -> ok\n", false),
        ("missing_arrow", "-module(sample).\nf() ok.\n", false),
        ("unclosed_list", "-module(sample).\nf() -> [1, 2.\n", false),
        ("missing_expression", "-module(sample).\nf() -> .\n", false),
        (
            "unclosed_string",
            "-module(sample).\nf() -> \"oops.\n",
            false,
        ),
    ]
    .into_iter()
    .map(|(name, source, valid)| (name.to_owned(), source.to_owned(), valid))
    .collect::<Vec<_>>();
    // 固定源文件语料覆盖分号、注释、字符串/字符中的点，以及 Unicode/CRLF。
    // 预处理例仅做候选解析和 erlc 对照，不冒充单文件原生 forms 已完成。
    let extra: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/erlang_source_forms.json"
    ))
    .unwrap();
    for case in extra.as_array().unwrap() {
        cases.push((
            case["name"].as_str().unwrap().to_owned(),
            case["source"].as_str().unwrap().to_owned(),
            case["valid"].as_bool().unwrap(),
        ));
    }
    cases
}

fn candidate_validity(root: &Path, source: &str) -> Option<bool> {
    let file = root.join("sample.erl");
    fs::write(&file, source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "erlang"])
        .arg(&file)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["reason"], serde_json::Value::Null, "{report}");
    if report["precheck"]["truncated_files"].as_u64().unwrap() > 0 {
        None
    } else {
        Some(report["recoveries"].as_array().unwrap().is_empty())
    }
}

#[test]
fn erlang_candidate_retains_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-erlang-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut disagreements = Vec::new();
    let mut unresolved = Vec::new();
    for (name, source, expected_valid) in corpus() {
        match candidate_validity(&root, &source) {
            Some(valid) if valid != expected_valid => disagreements.push(name),
            None => unresolved.push(name),
            _ => {}
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert!(
        disagreements.is_empty(),
        "syntax disagreements: {disagreements:?}"
    );
    assert!(unresolved.is_empty(), "unresolved: {unresolved:?}");
}

#[test]
#[ignore = "requires existing OTP 28 erlc and erl via CODEGUARD_ERLC_BIN and CODEGUARD_ERL_BIN"]
fn pinned_erlang_worker_matches_native_compiler() {
    let erlc = std::env::var("CODEGUARD_ERLC_BIN").expect("existing erlc executable");
    let erl = std::env::var("CODEGUARD_ERL_BIN").expect("existing erl executable");
    let version = Command::new(&erl)
        .args([
            "-noshell",
            "-eval",
            "io:format(\"OTP ~s~n\", [erlang:system_info(otp_release)]), halt().",
        ])
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), "OTP 28");
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-erlang-native-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut disagreements = Vec::new();
    let mut unresolved = Vec::new();
    for (name, source, expected_valid) in corpus() {
        let file = root.join("sample.erl");
        fs::write(&file, &source).unwrap();
        let output_dir = root.join(format!("out-{name}"));
        fs::create_dir_all(&output_dir).unwrap();
        let native = Command::new(&erlc)
            .arg("-o")
            .arg(&output_dir)
            .arg(&file)
            .output()
            .unwrap();
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
        let lint = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["lint", "erlang"])
            .arg(&file)
            .args(["--erl-tool", &erl, "--format=json"])
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert_eq!(lint.status.code(), Some(3), "{name}: {lint:?}");
        let feedback: serde_json::Value = serde_json::from_slice(&lint.stdout).unwrap();
        assert_eq!(
            feedback["native"]["status"],
            if matches!(name.as_str(), "macro" | "conditional_forms") {
                "incomplete"
            } else if expected_valid {
                "completed"
            } else {
                "diagnostics_observed"
            },
            "{name}: {feedback}"
        );
        assert_eq!(feedback["syntax_precheck"], serde_json::Value::Null);
        assert_eq!(feedback["delivery_decision"], "not_evaluated");
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
        match candidate_validity(&root, &source) {
            Some(valid) if valid != native.status.success() => disagreements.push(name),
            None => unresolved.push(name),
            _ => {}
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert!(
        disagreements.is_empty(),
        "syntax disagreements: {disagreements:?}"
    );
    assert!(unresolved.is_empty(), "unresolved: {unresolved:?}");
}
