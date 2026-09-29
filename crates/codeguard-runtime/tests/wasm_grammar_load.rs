#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::WasmGrammar;

const JAVA: &[u8] = include_bytes!("../../../grammars/java/parser.wasm");
const PYTHON: &[u8] = include_bytes!("../../../grammars/python/parser.wasm");
const TYPESCRIPT: &[u8] = include_bytes!("../../../grammars/typescript/parser.wasm");
const TSX: &[u8] = include_bytes!("../../../grammars/tsx/parser.wasm");

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
}

#[test]
fn invalid_bytes_hash_and_abi_are_not_accepted() {
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
