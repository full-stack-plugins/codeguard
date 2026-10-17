#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::{WasmGrammar, scan_wasm_sibling_bindings};

fn javascript() -> WasmGrammar {
    WasmGrammar::load(
        "javascript",
        include_bytes!("../../../grammars/javascript/parser.wasm"),
        "7978e62bcc851ab1d1f6dcd4678f9eda79df2b3b3490e75f81dd819d9bccccfa",
        15,
    )
    .unwrap()
}

fn kinds() -> [&'static str; 4] {
    [
        "program",
        "lexical_declaration",
        "variable_declarator",
        "identifier",
    ]
}

#[test]
fn duplicate_direct_lexical_names_are_ast_facts_without_raw_parser_errors() {
    let mut grammar = javascript();
    for source in [
        "const x = 1; const x = 2;\n",
        "let x; const x = 1;\n",
        "const x=1, x=2;\n",
        "let 名字;\r\nconst 名字 = 1;\r\n",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error());
        let scan =
            scan_wasm_sibling_bindings(&tree, source.as_bytes(), kinds(), 128, 200_000).unwrap();
        assert_eq!(scan.duplicates.len(), 1, "{source}");
        assert!(!scan.truncated);
        let fact = &scan.duplicates[0];
        assert!(fact.start_byte < fact.end_byte);
        assert!(source.is_char_boundary(fact.start_byte));
        assert!(source.is_char_boundary(fact.end_byte));
        assert_eq!(fact.parent_syntax_kind, "program");
    }
}

#[test]
fn distinct_scopes_var_redeclarations_and_text_do_not_create_duplicate_facts() {
    let mut grammar = javascript();
    for source in [
        "let x; { let x; }",
        "let x; function f() { let x; }",
        "var x; var x;",
        "const x = 'const x = 2'; // const x = 3\n",
        "const {x} = value; const x = 2;",
        "return 1;",
        "let x; let y;",
        "export const x=1; const x=2;",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        let scan =
            scan_wasm_sibling_bindings(&tree, source.as_bytes(), kinds(), 128, 200_000).unwrap();
        assert!(scan.duplicates.is_empty(), "{source}");
        assert!(!scan.truncated);
    }
}

#[test]
fn bounded_scan_retains_facts_but_never_hides_incomplete_traversal() {
    let mut grammar = javascript();
    let source = b"let x; let x; let x;";
    let tree = grammar.parse(source).unwrap();
    let scan = scan_wasm_sibling_bindings(&tree, source, kinds(), 1, 200_000).unwrap();
    assert_eq!(scan.duplicates.len(), 1);
    assert!(scan.truncated);
    let scan = scan_wasm_sibling_bindings(&tree, source, kinds(), 128, 1).unwrap();
    assert!(scan.duplicates.is_empty());
    assert!(scan.truncated);
    assert!(scan_wasm_sibling_bindings(&tree, source, kinds(), 0, 200_000).is_err());
    assert!(scan_wasm_sibling_bindings(&tree, source, kinds(), 128, 0).is_err());
    assert!(scan_wasm_sibling_bindings(&tree, b"", kinds(), 128, 200_000).is_err());
    assert!(
        scan_wasm_sibling_bindings(
            &tree,
            source,
            [
                "wrong",
                "lexical_declaration",
                "variable_declarator",
                "identifier"
            ],
            128,
            200_000
        )
        .is_err()
    );
}

#[test]
#[ignore = "需要显式已安装Node24.18.0；原生模块/CommonJS对照，不批准语言资格"]
fn real_node_confirms_duplicate_facts_without_guessing_module_return_semantics() {
    use codeguard_runtime::{ProcessSpec, Termination, run_process};
    use std::{
        collections::BTreeMap,
        ffi::OsString,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").expect("显式指定Node"));
    assert!(tool.is_absolute());
    let executable = tool.canonicalize().unwrap();
    let before = std::fs::read(&executable).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let cancelled = AtomicBool::new(false);
    let invoke = |args: &[&str], stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args: args.iter().map(OsString::from).collect(),
                cwd: Path::new("/").to_path_buf(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 65536,
            },
            &cancelled,
        )
    };
    let version = invoke(&["--version"], None);
    assert_eq!(version.termination, Termination::Exited(0));
    assert_eq!(version.stdout, b"v24.18.0\n");
    assert!(version.stderr.is_empty());
    let mut grammar = javascript();
    for mode in ["--input-type=module", "--input-type=commonjs"] {
        for (source, duplicate) in [
            ("const x=1; const x=2;", true),
            ("let x; const x=1;", true),
            ("let x; {let x;}", false),
            ("let x; function f(){let x;}", false),
            ("var x; var x;", false),
            ("const x='let x;'; // let x;\n", false),
            ("let 名字;\r\nconst 名字=1;", true),
            ("return 1;", false),
        ] {
            let outcome = invoke(&["--check", mode], Some(source.as_bytes().to_vec()));
            let invalid = duplicate || (source == "return 1;" && mode == "--input-type=module");
            assert_eq!(
                outcome.termination,
                Termination::Exited(i32::from(invalid)),
                "{mode} {source}"
            );
            assert!(outcome.stdout.is_empty());
            if invalid {
                assert!(String::from_utf8_lossy(&outcome.stderr).contains("SyntaxError"));
            } else {
                assert!(outcome.stderr.is_empty());
            }
            let tree = grammar.parse(source.as_bytes()).unwrap();
            let scan = scan_wasm_sibling_bindings(&tree, source.as_bytes(), kinds(), 128, 200_000)
                .unwrap();
            assert_eq!(!scan.duplicates.is_empty(), duplicate, "{mode} {source}");
            assert!(!scan.truncated);
        }
    }
    assert_eq!(tool.canonicalize().unwrap(), executable);
    assert_eq!(std::fs::read(executable).unwrap(), before);
}
