#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::WasmGrammar;

const JAVA: &[u8] = include_bytes!("../../../grammars/java/parser.wasm");
const PYTHON: &[u8] = include_bytes!("../../../grammars/python/parser.wasm");
const TYPESCRIPT: &[u8] = include_bytes!("../../../grammars/typescript/parser.wasm");
const TSX: &[u8] = include_bytes!("../../../grammars/tsx/parser.wasm");
const ZIG: &[u8] = include_bytes!("../../../grammars/zig/parser.wasm");
const ZIG_SOURCE: &[u8] = include_bytes!("../../../grammars/zig/source.wasm");
const OBJC: &[u8] = include_bytes!("../../../grammars/objc/parser.wasm");
const SOLIDITY: &[u8] = include_bytes!("../../../grammars/solidity/parser.wasm");
const C: &[u8] = include_bytes!("../../../grammars/c/parser.wasm");
const GO: &[u8] = include_bytes!("../../../grammars/go/parser.wasm");
const JAVASCRIPT: &[u8] = include_bytes!("../../../grammars/javascript/parser.wasm");
const RUST: &[u8] = include_bytes!("../../../grammars/rust/parser.wasm");
const CPP: &[u8] = include_bytes!("../../../grammars/cpp/parser.wasm");
const CSHARP: &[u8] = include_bytes!("../../../grammars/csharp/parser.wasm");
const LUA: &[u8] = include_bytes!("../../../grammars/lua/parser.wasm");
const LUAU: &[u8] = include_bytes!("../../../grammars/luau/parser.wasm");
const ARKTS: &[u8] = include_bytes!("../../../grammars/arkts/parser.wasm");
const NIX: &[u8] = include_bytes!("../../../grammars/nix/parser.wasm");
const TERRAFORM: &[u8] = include_bytes!("../../../grammars/terraform/parser.wasm");
const R: &[u8] = include_bytes!("../../../grammars/r/parser.wasm");
const RUBY: &[u8] = include_bytes!("../../../grammars/ruby/parser.wasm");
const PHP: &[u8] = include_bytes!("../../../grammars/php/parser.wasm");
const KOTLIN: &[u8] = include_bytes!("../../../grammars/kotlin/parser.wasm");
const DART: &[u8] = include_bytes!("../../../grammars/dart/parser.wasm");
const DART_REBUILT_SOURCE: &[u8] = include_bytes!("../../../grammars/dart/source.wasm");
const ERLANG: &[u8] = include_bytes!("../../../grammars/erlang/parser.wasm");
const PASCAL: &[u8] = include_bytes!("../../../grammars/pascal/parser.wasm");
const COBOL: &[u8] = include_bytes!("../../../grammars/cobol/parser.wasm");
const CFML: &[u8] = include_bytes!("../../../grammars/cfml/parser.wasm");
const CFQUERY: &[u8] = include_bytes!("../../../grammars/cfquery/parser.wasm");
const CFSCRIPT: &[u8] = include_bytes!("../../../grammars/cfscript/parser.wasm");
const SCALA: &[u8] = include_bytes!("../../../grammars/scala/parser.wasm");
const SWIFT: &[u8] = include_bytes!("../../../grammars/swift/parser.wasm");
const VBNET: &[u8] = include_bytes!("../../../grammars/vbnet/parser.wasm");

#[test]
fn final_source_grammars_load_offline_and_separate_basic_syntax() {
    for (name, wasm, sha, valid, invalid) in [
        (
            "cfml",
            CFML,
            "17aeb294c58dd47045b470fc082ccceac3100c8e548b13764d4f3987a062322f",
            b"<cfset x = 1>".as_slice(),
            b"<cfif>".as_slice(),
        ),
        (
            "cfquery",
            CFQUERY,
            "9d4eaaec46eec7e6400d4b8d6c85fac7dfe33867ed5b4e5c218dfa7851c32558",
            b"SELECT * FROM users".as_slice(),
            b"SELECT #unclosed".as_slice(),
        ),
        (
            "cfscript",
            CFSCRIPT,
            "e381db5d2b8a7744fc7d8e51b0a14ff03f86c8ff497cdfe32d1da467f19d4096",
            b"component { function f() { return 1; } }".as_slice(),
            b"component { function f( {".as_slice(),
        ),
        (
            "scala",
            SCALA,
            "37d7fe5a91ca98941dc05493b0c05a0df0f36df5035890fa00b02497c68aaac3",
            b"object Main { def f(x: Int): Int = x + 1 }".as_slice(),
            b"object Main { def f( = }".as_slice(),
        ),
        (
            "swift",
            SWIFT,
            "cc77a63b8487956270e2f385e29a03ba0773ba532a3c8a8844a26b4c98793843",
            b"func f() -> Int { return 1 }".as_slice(),
            b"func f( {".as_slice(),
        ),
        (
            "vbnet",
            VBNET,
            "e38a09e1826c7ec06340c0531a98bbb5ee79bdd710bba3d3349b79a19721e844",
            b"Public Class C\n    Public Function F() As Integer\n        Return 1\n    End Function\nEnd Class\n"
                .as_slice(),
            b"Public Class C\nPublic Function F(\n".as_slice(),
        ),
    ] {
        let mut grammar = WasmGrammar::load(name, wasm, sha, 15)
            .unwrap_or_else(|reason| panic!("{name} offline load: {reason}"));
        assert!(
            !grammar.parse(valid).unwrap().root_node().has_error(),
            "{name} valid"
        );
        assert!(
            grammar.parse(invalid).unwrap().root_node().has_error(),
            "{name} invalid"
        );
    }
}

#[test]
fn pinned_cobol_asset_loads_with_its_original_export_name() {
    let mut grammar = WasmGrammar::load(
        "COBOL",
        COBOL,
        "65b98799f92e831d8b5ed8cd811464c71fac31c4beabf66990a545a4f65e02f1",
        14,
    )
    .expect("pinned COBOL grammar under bounded worker input");
    assert!(
        !grammar
            .parse(b"       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn pascal_grammar_loads_offline_and_separates_basic_syntax() {
    let mut grammar = WasmGrammar::load(
        "pascal",
        PASCAL,
        "be3634fca99c19f5e1035a1a9c7d93d6ee82b35e6d5024f02be4883b71329c3e",
        14,
    )
    .expect("pinned Pascal grammar");
    assert!(
        !grammar
            .parse(b"program Hello;\nbegin\n  writeln('Hi');\nend.\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        grammar
            .parse(b"program Hello;\nbegin\n  writeln('Hi');\n")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn erlang_grammar_loads_offline_and_separates_basic_syntax() {
    let mut grammar = WasmGrammar::load(
        "erlang",
        ERLANG,
        "dbab33f03e07b89f4385fcdd48d87d86ba35c82a0a426788d55b8c25410bc491",
        14,
    )
    .expect("pinned Erlang grammar");
    assert!(
        !grammar
            .parse(b"-module(hello).\nhello() -> ok.\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        grammar
            .parse(b"-module(hello).\nhello( -> ok.\n")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn rebuilt_dart_grammar_loads_with_real_scanner_and_rejects_broken_source() {
    assert!(
        WasmGrammar::load(
            "dart",
            DART_REBUILT_SOURCE,
            "bbb37cc6aebca30188fab912bb2ae7201604d2b001a64ea6ef2c397674f81f5c",
            15,
        )
        .is_err()
    );
    let mut grammar = WasmGrammar::load(
        "dart",
        DART,
        "7dad281b3b24924d619cb7059a42b409e7690ebeb8ebbc82882e68167656e012",
        15,
    )
    .expect("rebuilt Dart grammar with compiled scanner");
    for source in [
        "void main() { print('hi'); }",
        "/// doc\nString greet(String name) => 'Hello $name';",
        "/* block */\nvoid f() {}",
        "String nested = 'hi ${1 + 2}';",
    ] {
        assert!(
            !grammar
                .parse(source.as_bytes())
                .unwrap()
                .root_node()
                .has_error(),
            "valid Dart: {source}"
        );
    }
    for source in ["void main( {", "String x = 'unfinished", "/* unclosed"] {
        assert!(
            grammar
                .parse(source.as_bytes())
                .unwrap()
                .root_node()
                .has_error(),
            "broken Dart: {source}"
        );
    }
}

#[test]
fn fourth_batch_grammars_load_offline_and_parse_basic_fixtures() {
    for (name, wasm, sha, abi, valid, invalid) in [
        (
            "r",
            R,
            "2a8f5acd1c53d91e0ec5c01a6830d8ac7f5a7f96f0ac4b3768c016c8e9d07711",
            14,
            b"x <- 1\n".as_slice(),
            b"x <- \n".as_slice(),
        ),
        (
            "ruby",
            RUBY,
            "4cb5a4b12870876ca864c1e92fe1f5cd47036b2adc083e9306488af88867dbb4",
            14,
            b"def f; 1; end\n".as_slice(),
            b"def f(\n".as_slice(),
        ),
        (
            "php",
            PHP,
            "6545a9a110bc878e26ed329950147e190c83da038bb17e999de646fe6c4d6c82",
            15,
            b"<?php function f() { return 1; }".as_slice(),
            b"<?php function f( {".as_slice(),
        ),
        (
            "kotlin",
            KOTLIN,
            "c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48",
            14,
            b"fun main() { val x = 1 }".as_slice(),
            b"fun main( {".as_slice(),
        ),
    ] {
        let mut grammar = WasmGrammar::load(name, wasm, sha, abi)
            .unwrap_or_else(|reason| panic!("{name} offline load: {reason}"));
        assert!(
            !grammar.parse(valid).unwrap().root_node().has_error(),
            "{name} valid"
        );
        assert!(
            grammar.parse(invalid).unwrap().root_node().has_error(),
            "{name} invalid"
        );
    }
    let mut php = WasmGrammar::load(
        "php",
        PHP,
        "6545a9a110bc878e26ed329950147e190c83da038bb17e999de646fe6c4d6c82",
        15,
    )
    .expect("full PHP grammar");
    assert!(
        !php.parse(b"<div>Hi</div><?php echo 1; ?>")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn third_batch_grammars_load_offline_and_parse_basic_fixtures() {
    for (name, wasm, sha, abi, valid, invalid) in [
        (
            "arkts",
            ARKTS,
            "db0812971109457d22b3fe9dcdb1cd8e614fba2a338e220670bd254485753623",
            14,
            b"@Component struct C { build() { Text('hi') } }".as_slice(),
            b"@Component struct C { build( {".as_slice(),
        ),
        (
            "nix",
            NIX,
            "4acffa1c013df751193a21ce429777ca214c7d3a7e3665085b6df00dad8869f0",
            15,
            b"let x = 1; in x".as_slice(),
            b"let x = ; in x".as_slice(),
        ),
        (
            "terraform",
            TERRAFORM,
            "0d9ef3ae926acc0411bfecba9b34df4ea659061917b96455570a45a70824e890",
            14,
            b"resource \"x\" \"y\" { foo = \"bar\" }".as_slice(),
            b"resource \"x\" {".as_slice(),
        ),
    ] {
        let mut grammar = WasmGrammar::load(name, wasm, sha, abi)
            .unwrap_or_else(|reason| panic!("{name} offline load: {reason}"));
        assert!(
            !grammar.parse(valid).unwrap().root_node().has_error(),
            "{name} valid"
        );
        assert!(
            grammar.parse(invalid).unwrap().root_node().has_error(),
            "{name} invalid"
        );
    }
}

#[test]
fn second_batch_grammars_load_offline_and_parse_basic_fixtures() {
    for (name, wasm, sha, abi, valid, invalid) in [
        (
            "cpp",
            CPP,
            "70f5e2b9976dad56bdcd1fafcb3af8c839c7a92e7beaa437162bcf45f390e83d",
            14,
            b"int main() { return 0; }".as_slice(),
            b"int main() {".as_slice(),
        ),
        (
            "c_sharp",
            CSHARP,
            "6f69e1cae44e1c32c1eccc170dc5a9778fb94ff716f71113fe1f8c4299aa2f40",
            15,
            b"class C { static int F() { return 1; } }".as_slice(),
            b"class C {".as_slice(),
        ),
        (
            "lua",
            LUA,
            "6d95607fc7d78964cfdf065ccb1ba76be5ed217c5ec0d0a3cace13c59fa1ae43",
            15,
            b"local x = 1\n".as_slice(),
            b"local =\n".as_slice(),
        ),
        (
            "luau",
            LUAU,
            "f1647052518f2bdfae8e8c0b033ffdeca1193d69d11c78ba20f84c8374fd0fe3",
            14,
            b"local x: number = 1\n".as_slice(),
            b"local x: =\n".as_slice(),
        ),
    ] {
        let mut grammar = WasmGrammar::load(name, wasm, sha, abi)
            .unwrap_or_else(|reason| panic!("{name} offline load: {reason}"));
        assert!(
            !grammar.parse(valid).unwrap().root_node().has_error(),
            "{name} valid"
        );
        assert!(
            grammar.parse(invalid).unwrap().root_node().has_error(),
            "{name} invalid"
        );
    }
}

#[test]
fn mainstream_grammars_load_offline_and_parse_basic_fixtures() {
    for (language, wasm, sha, abi, valid, invalid) in [
        (
            "c",
            C,
            "a271e584616c7c3c0ac663f01cd05dd5f1a6c2ce6d4cd23096548985a95d0ccb",
            15,
            b"int main(void) { return 0; }".as_slice(),
            b"int main(void) {".as_slice(),
        ),
        (
            "go",
            GO,
            "4eda5d91c99ca981e88bc7d3d33f0db166b4bab0a84d0021a9abf39b364c78ef",
            14,
            b"package main\nfunc main() {}\n".as_slice(),
            b"package main\nfunc main( {\n".as_slice(),
        ),
        (
            "javascript",
            JAVASCRIPT,
            "7978e62bcc851ab1d1f6dcd4678f9eda79df2b3b3490e75f81dd819d9bccccfa",
            15,
            b"const value = 1;".as_slice(),
            b"const = ;".as_slice(),
        ),
        (
            "rust",
            RUST,
            "206031e0f67fb41ecae505868ca3bb917df7375031aebafd2f97314a849713fe",
            15,
            b"fn main() { let x = 1; }".as_slice(),
            b"fn main( {".as_slice(),
        ),
    ] {
        let mut grammar = WasmGrammar::load(language, wasm, sha, abi)
            .unwrap_or_else(|reason| panic!("{language} offline load: {reason}"));
        assert!(
            !grammar.parse(valid).unwrap().root_node().has_error(),
            "{language} valid"
        );
        assert!(
            grammar.parse(invalid).unwrap().root_node().has_error(),
            "{language} invalid"
        );
    }
}

#[test]
fn dependency_grammars_load_offline_and_parse_basic_fixtures() {
    let mut objc = WasmGrammar::load(
        "objc",
        OBJC,
        "2606d4c5809fab61de44d328072d1371d53aa4aff5734436cb2a0ce8db7f2b0c",
        14,
    )
    .expect("Objective-C dependency grammar loads in Rust");
    assert!(
        !objc
            .parse(b"@interface Foo : NSObject\n@end\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        objc.parse(b"@interface Foo : NSObject\n")
            .unwrap()
            .root_node()
            .has_error()
    );

    let mut solidity = WasmGrammar::load(
        "solidity",
        SOLIDITY,
        "ba02ba3c98c8ce976ed962d727ef48940b3a18dd2243830a8158b558de64b4f2",
        14,
    )
    .expect("Solidity dependency grammar loads in Rust");
    assert!(
        !solidity
            .parse(b"pragma solidity ^0.8.20;\ncontract Vault {}\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        solidity
            .parse(b"contract Vault {\n")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn pinned_grammars_load_offline_and_parse_valid_sources() {
    let mut python = WasmGrammar::load(
        "python",
        PYTHON,
        "a7fdc587e77bd729b9f5b783c659be23c896e305a2c374472bed7114d9e01fac",
        14,
    )
    .expect("固定 Python WASM 应可由 Rust 加载");
    assert_eq!(python.abi_version(), 14);
    assert!(
        !python
            .parse(b"def greet(name):\n    return name\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        python
            .parse(b"def greet(\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    let mut java = WasmGrammar::load(
        "java",
        JAVA,
        "181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077",
        14,
    )
    .expect("固定 Java WASM 应可由 Rust 加载");
    assert_eq!(java.abi_version(), 14);
    assert!(
        !java
            .parse(b"class Example { int value = 1; }")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        java.parse(b"class Example {")
            .unwrap()
            .root_node()
            .has_error()
    );

    let mut typescript = WasmGrammar::load(
        "typescript",
        TYPESCRIPT,
        "3a44d634c9840dccec36f33b99592bd086a2f940e9cd80c64347c45b7dce662f",
        14,
    )
    .expect("固定 TypeScript WASM 应可由 Rust 加载");
    assert_eq!(typescript.abi_version(), 14);
    assert!(
        !typescript
            .parse(b"const value: number = 1;")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(typescript.parse(&vec![b'x'; 1024 * 1024 + 1]).is_err());

    let mut tsx = WasmGrammar::load(
        "tsx",
        TSX,
        "8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e",
        14,
    )
    .expect("固定 TSX WASM 应可由 Rust 加载");
    assert_eq!(tsx.abi_version(), 14);
    assert!(
        !tsx.parse(b"const node = <div title=\"ok\">hello</div>;")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        tsx.parse(b"const node = <div title=></div>;")
            .unwrap()
            .root_node()
            .has_error()
    );

    let mut zig = WasmGrammar::load(
        "zig",
        ZIG,
        "e8a3aa89cc07b59188122e6c1e0a812ca58cf85c9e191dcf2663a30b6b3b99e3",
        15,
    )
    .expect("patched Zig should load");
    assert_eq!(zig.abi_version(), 15);
    assert!(
        !zig.parse(b"const S = struct {};\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        !zig.parse(b"const S = struct {};\nconst E = enum {};\nconst U = union(enum) {};\nconst O = opaque {};\n")
            .unwrap()
            .root_node()
            .has_error()
    );
    assert!(
        zig.parse(b"const S = struct {\n")
            .unwrap()
            .root_node()
            .has_error()
    );
}

#[test]
fn invalid_bytes_hash_and_abi_are_not_accepted() {
    let original_zig = WasmGrammar::load(
        "zig",
        ZIG_SOURCE,
        "95d8eef504bde9cca06b7950d6a8ae177ce318d16080deac96f653b29c629f98",
        15,
    )
    .err()
    .expect("原始 Zig WASM 与 Rust WasmStore 不兼容");
    assert!(original_zig.contains("__main_argc_argv"));
    assert!(WasmGrammar::load("java\0unsafe", JAVA, "0", 14).is_err());
    assert!(WasmGrammar::load("java", b"broken", "0", 14).is_err());
    let empty_module = b"\0asm\x01\0\0\0";
    let empty_module_sha256 = ring::digest::digest(&ring::digest::SHA256, empty_module)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert!(WasmGrammar::load("java", empty_module, &empty_module_sha256, 14).is_err());
    assert!(
        WasmGrammar::load(
            "java",
            JAVA,
            "0000000000000000000000000000000000000000000000000000000000000000",
            14,
        )
        .is_err()
    );
    let error = WasmGrammar::load(
        "java",
        JAVA,
        "181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077",
        13,
    )
    .err()
    .expect("错误 ABI 声明必须被拒绝");
    assert!(error.contains("ABI"));
}
