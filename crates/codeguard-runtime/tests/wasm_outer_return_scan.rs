#![cfg(feature = "wasm-precheck")]
use codeguard_runtime::{WasmGrammar, scan_wasm_outer_returns};

fn javascript() -> WasmGrammar {
    WasmGrammar::load(
        "javascript",
        include_bytes!("../../../grammars/javascript/parser.wasm"),
        "7978e62bcc851ab1d1f6dcd4678f9eda79df2b3b3490e75f81dd819d9bccccfa",
        15,
    )
    .unwrap()
}
fn scan(
    tree: &tree_sitter::Tree,
    source: &[u8],
    records: usize,
    visits: usize,
) -> codeguard_runtime::WasmOuterReturnScan {
    scan_wasm_outer_returns(
        tree,
        source,
        ["program", "return_statement"],
        &[
            "function_declaration",
            "function_expression",
            "generator_function_declaration",
            "generator_function",
            "arrow_function",
            "method_definition",
        ],
        records,
        visits,
    )
    .unwrap()
}
#[test]
fn outside_function_returns_include_control_flow_and_byte_positions() {
    let mut grammar = javascript();
    for source in [
        "return 1;",
        "if (ready) { return 1; }",
        "while (ready) { return 2; }",
        "try { return 1; } catch (e) { return 2; }",
        "const 名字=1;\r\nif (名字) { return 名字; }",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error(), "{source}");
        let facts = scan(&tree, source.as_bytes(), 128, 200_000);
        assert!(!facts.returns.is_empty(), "{source}");
        assert!(!facts.truncated);
        for fact in &facts.returns {
            assert!(source[fact.start_byte..fact.end_byte].starts_with("return"));
            assert_eq!(fact.syntax_kind, "return_statement");
            let prefix = &source.as_bytes()[..fact.start_byte];
            assert_eq!(
                fact.start_row,
                prefix.iter().filter(|b| **b == b'\n').count()
            );
            assert_eq!(
                fact.start_column_byte,
                prefix.rsplit(|b| *b == b'\n').next().unwrap().len()
            );
        }
    }
}
#[test]
fn every_function_boundary_and_comment_text_is_skipped() {
    let mut grammar = javascript();
    for source in [
        "function f(){if(true){return 1;}}",
        "const f = function(){return 1;};",
        "function* f(){return 1;}",
        "const f = function*(){return 1;};",
        "const f = () => {return 1;};",
        "class C { f(){return 1;} get x(){return 2;} }",
        "const o = {f(){return 1;}};",
        "async function f(){return 1;}",
        "const f = async () => {return 1;};",
        "const x = 'return 1;'; // return 2;\n",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error(), "{source}");
        let facts = scan(&tree, source.as_bytes(), 128, 200_000);
        assert!(facts.returns.is_empty(), "{source}");
        assert!(!facts.truncated);
    }
    let source = b"function f(){return 1;} if(true){return 2;}";
    let tree = grammar.parse(source).unwrap();
    let facts = scan(&tree, source, 128, 200_000);
    assert_eq!(facts.returns.len(), 1);
    assert_eq!(
        &source[facts.returns[0].start_byte..facts.returns[0].end_byte],
        b"return 2;"
    );
}
#[test]
fn exhausted_scan_retains_facts_and_invalid_bounds_are_rejected() {
    let mut grammar = javascript();
    let source = b"return 1; return 2;";
    let tree = grammar.parse(source).unwrap();
    let facts = scan(&tree, source, 1, 200_000);
    assert_eq!(facts.returns.len(), 1);
    assert!(facts.truncated);
    let facts = scan(&tree, source, 128, 1);
    assert!(facts.returns.is_empty());
    assert!(facts.truncated);
    for (root, ret, scopes, records, visits, bytes) in [
        (
            "program",
            "return_statement",
            &["function_declaration"][..],
            0,
            10,
            &source[..],
        ),
        (
            "program",
            "return_statement",
            &["function_declaration"][..],
            1,
            0,
            &source[..],
        ),
        (
            "wrong",
            "return_statement",
            &["function_declaration"][..],
            1,
            10,
            &source[..],
        ),
        (
            "program",
            "",
            &["function_declaration"][..],
            1,
            10,
            &source[..],
        ),
        ("program", "return_statement", &[][..], 1, 10, &source[..]),
        (
            "program",
            "return_statement",
            &["function_declaration"][..],
            1,
            10,
            &b""[..],
        ),
    ] {
        assert!(
            scan_wasm_outer_returns(&tree, bytes, [root, ret], scopes, records, visits).is_err()
        );
    }
}

#[test]
#[ignore = "需显式已有Node24.18.0，验证module/CommonJS模式边界，不批准语言资格"]
fn actual_node_modes_confirm_outer_return_facts_without_commonjs_findings() {
    use codeguard_runtime::{ProcessSpec, Termination, run_process};
    use std::{
        collections::BTreeMap,
        ffi::OsString,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let input = PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").expect("显式指定Node"));
    assert!(input.is_absolute());
    let executable = input.canonicalize().unwrap();
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
    for source in [
        "return 1;",
        "if(true){return 1;}",
        "while(false){return 1;}",
        "function f(){return 1;}",
        "const f=function(){return 1;};",
        "function* f(){return 1;}",
        "const f=function*(){return 1;};",
        "const f=()=>{return 1;};",
        "class C {f(){return 1;}}",
        "const o={f(){return 1;}};",
        "const s='return 1;';",
    ] {
        let tree = grammar.parse(source.as_bytes()).unwrap();
        assert!(!tree.root_node().has_error());
        let facts = scan(&tree, source.as_bytes(), 128, 200_000);
        assert!(!facts.truncated);
        for (mode, module) in [
            ("--input-type=module", true),
            ("--input-type=commonjs", false),
        ] {
            let outcome = invoke(&["--check", mode], Some(source.as_bytes().to_vec()));
            let invalid = module && !facts.returns.is_empty();
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
        }
    }
    assert_eq!(input.canonicalize().unwrap(), executable);
    assert_eq!(std::fs::read(executable).unwrap(), before);
}
