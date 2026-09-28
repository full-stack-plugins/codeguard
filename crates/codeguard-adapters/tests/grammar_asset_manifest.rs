use codeguard_adapters::{
    bundled_grammar_candidates, parse_grammar_asset_manifest, verify_grammar_asset,
};

#[test]
fn bundled_candidates_pin_source_bytes_license_and_abi_without_claiming_support() {
    let manifest = bundled_grammar_candidates().expect("bundled manifest");
    assert_eq!(manifest.assets.len(), 2);
    for asset in &manifest.assets {
        let (wasm, license) = match asset.language.as_str() {
            "java" => (
                include_bytes!("../../../grammars/java/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/java/LICENSE").as_slice(),
            ),
            "typescript" => (
                include_bytes!("../../../grammars/typescript/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
            ),
            other => panic!("unexpected language {other}"),
        };
        verify_grammar_asset(asset, wasm, license).expect("pinned candidate bytes");
        assert_eq!(asset.release_status, "candidate_unvalidated");
        assert_eq!(
            asset.codeguard_runtime_validation,
            "rust_loader_smoke_passed"
        );
        assert!(asset.language_versions.is_empty());
    }
}

#[test]
fn missing_provenance_duplicate_fields_and_unknown_languages_are_rejected() {
    let original = include_str!("../../../grammars/manifest.json");
    let missing = original.replacen("94703d5a6bed02b98e438d7cad1136c01a60ba2c", "", 1);
    assert!(parse_grammar_asset_manifest(missing.as_bytes()).is_err());
    let duplicate = original.replacen(
        "\"schema_version\": \"1.0.0\",",
        "\"schema_version\": \"1.0.0\", \"schema_version\": \"1.0.0\",",
        1,
    );
    assert!(parse_grammar_asset_manifest(duplicate.as_bytes()).is_err());
    let unknown = original.replacen("\"language\": \"java\"", "\"language\": \"zig\"", 1);
    assert!(parse_grammar_asset_manifest(unknown.as_bytes()).is_err());
}

#[test]
fn changed_wasm_license_or_abi_never_qualifies_a_candidate() {
    let manifest = bundled_grammar_candidates().unwrap();
    let mut asset = manifest.assets[0].clone();
    let wasm = include_bytes!("../../../grammars/java/parser.wasm");
    let license = include_bytes!("../../../grammars/java/LICENSE");
    let mut changed = wasm.to_vec();
    changed[64] ^= 1;
    assert!(verify_grammar_asset(&asset, &changed, license).is_err());
    assert!(verify_grammar_asset(&asset, wasm, b"unlicensed").is_err());
    asset.abi_version = 99;
    assert!(verify_grammar_asset(&asset, wasm, license).is_err());
    asset.abi_version = 14;
    asset.release_status = "supported".into();
    assert!(verify_grammar_asset(&asset, wasm, license).is_err());
    asset.sha256 = "0".repeat(64);
    assert!(verify_grammar_asset(&asset, wasm, license).is_err());
}
