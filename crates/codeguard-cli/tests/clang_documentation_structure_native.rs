use codeguard_runtime::{ProcessSpec, Termination, run_process};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires existing fixed Apple Clang21; set CODEGUARD_CLANG_BIN"]
fn actual_clang_ast_exposes_the_zero_warning_documentation_blind_spot() {
    let tool =
        PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("explicit installed tool"));
    assert!(tool.is_absolute());
    let tool = tool.canonicalize().unwrap();
    let original = std::fs::read(&tool).unwrap();
    let cwd = std::env::temp_dir().canonicalize().unwrap();
    let invoke = |args: Vec<OsString>, stdin| {
        run_process(
            &ProcessSpec {
                executable: tool.clone(),
                args,
                cwd: cwd.clone(),
                env: BTreeMap::new(),
                stdin,
                deadline: Instant::now() + Duration::from_secs(10),
                output_limit_bytes: 8 * 1024 * 1024,
            },
            &AtomicBool::new(false),
        )
    };
    let version = invoke(vec![OsString::from("--version")], None);
    assert_eq!(version.termination, Termination::Exited(0));
    assert!(
        version
            .stdout
            .starts_with(b"Apple clang version 21.0.0 (clang-2100.3.34.2)\n")
    );
    let mut cases = vec![
        (
            "absent",
            "int f(int x) { return x; }",
            vec!["documentation_comment"],
        ),
        (
            "empty",
            "/** */\nint f(int x) { return x; }",
            vec!["purpose", "parameter:x", "return"],
        ),
        (
            "purpose_only",
            "/** Compute square. */\nint f(int x) { return x*x; }",
            vec!["parameter:x", "return"],
        ),
        (
            "complete",
            "/** Compute square.\n * @param x input integer\n * @return square of input\n */\nint f(int x) { return x*x; }",
            vec![],
        ),
        (
            "unicode",
            "/** 计算平方。\n * @param x 输入整数。\n * @return 输入的平方。\n */\nint f(int x) { return x*x; }",
            vec![],
        ),
        (
            "brief_void",
            "/** @brief 执行初始化。 */\nvoid f(void) {}",
            vec![],
        ),
        (
            "blank_param",
            "/** Calculate.\n * @param x\n * @return result\n */\nint f(int x) { return x; }",
            vec!["parameter:x"],
        ),
        (
            "unnamed",
            "/** Read value.\n * @return value\n */\nint f(int);",
            vec![],
        ),
        (
            "alias",
            "typedef int Result;\n/** Compute result.\n * @param x input\n */\nResult f(int x) { return x; }",
            vec![],
        ),
        (
            "redeclared",
            "/** Read input.\n * @param x input\n * @return input\n */\nint f(int x);\nint f(int x) { return x; }",
            vec![],
        ),
    ];
    cases.push(("copydoc", "/** @copydoc original */\nint f(int x);", vec![]));
    let mut evidence = Vec::new();
    for (language, standard) in [("c", "c11"), ("c++", "c++17")] {
        for (name, source, expected) in &cases {
            let argv = [
                "--no-default-config",
                "-nostdinc",
                "-fsyntax-only",
                "-Wdocumentation",
                "-Xclang",
                "-ast-dump=json",
                "-x",
                language,
                "-",
            ];
            let mut args: Vec<OsString> = argv.into_iter().map(OsString::from).collect();
            args.insert(1, OsString::from(format!("-std={standard}")));
            let output = invoke(args, Some(source.as_bytes().to_vec()));
            assert_eq!(
                output.termination,
                Termination::Exited(0),
                "{language}/{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let parsed = codeguard_adapters::parse_clang_documentation_ast(
                &output.stdout,
                source.as_bytes(),
            )
            .unwrap();
            let rows = parsed["functions"].as_array().unwrap();
            assert!(!rows.is_empty());
            assert_eq!(
                rows[0]["missing_components"],
                json!(expected),
                "{language}/{name}"
            );
            if *name == "absent" {
                assert!(output.stderr.is_empty());
            }
            if *name == "alias" {
                assert_eq!(rows[0]["return_description"], "unknown");
            }
            if *name == "unnamed" {
                assert_eq!(rows[0]["parameters"][0]["description"], "unknown_unnamed");
            }
            if *name == "copydoc" {
                assert_eq!(rows[0]["structure_status"], "unresolved_comment_structure");
            }
            if *name == "redeclared" {
                assert_eq!(rows[1]["comment_presence"], "unknown_redeclaration");
                assert_eq!(rows[1]["missing_components"], json!([]));
            }
            evidence.push(json!({"language":language,"case":name,"standard":standard,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"ast_sha256":format!("{:x}",Sha256::digest(&output.stdout)),"native_stderr_empty":output.stderr.is_empty(),"observation":parsed}));
        }
    }
    for newline in ["\n", "\r", "\r\n", "\n\r"] {
        for (language, standard) in [("c", "c11"), ("c++", "c++17")] {
            let source = format!("/* 前缀 */{newline}int f(int x);");
            let args = [
                "--no-default-config",
                "-nostdinc",
                "-fsyntax-only",
                "-Wdocumentation",
                "-Xclang",
                "-ast-dump=json",
                "-x",
                language,
                "-",
            ];
            let mut argv: Vec<OsString> = args.into_iter().map(OsString::from).collect();
            argv.insert(1, OsString::from(format!("-std={standard}")));
            let output = invoke(argv, Some(source.as_bytes().to_vec()));
            assert_eq!(output.termination, Termination::Exited(0));
            let parsed = codeguard_adapters::parse_clang_documentation_ast(
                &output.stdout,
                source.as_bytes(),
            )
            .unwrap();
            let native: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let decl = native["inner"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["kind"] == "FunctionDecl")
                .unwrap();
            assert_eq!(parsed["functions"][0]["line"], decl["loc"]["line"]);
            assert_eq!(parsed["functions"][0]["column_byte"], decl["loc"]["col"]);
            evidence.push(json!({"language":language,"case":"newline","newline":newline,"source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"ast_sha256":format!("{:x}",Sha256::digest(&output.stdout)),"observation":parsed}));
        }
    }
    assert_eq!(std::fs::read(&tool).unwrap(), original);
    if let Some(path) = std::env::var_os("CODEGUARD_AST_EVIDENCE") {
        let value = json!({"evidence_kind":"development_native_ast_contract","qualification":"not_granted","independent_holdout":false,"adapter_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("../../codeguard-adapters/src/clang_documentation_ast.rs"))),"tool_sha256":format!("{:x}",Sha256::digest(original)),"test_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("clang_documentation_structure_native.rs"))),"cases":evidence});
        std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
}
