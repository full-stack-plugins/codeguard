#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::WasmGrammar;

const JAVA: &[u8] = include_bytes!("../../../grammars/java/parser.wasm");
const TYPESCRIPT: &[u8] = include_bytes!("../../../grammars/typescript/parser.wasm");

#[test]
fn pinned_grammars_load_offline_and_parse_valid_sources() {
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
