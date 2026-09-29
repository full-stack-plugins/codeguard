use codeguard_adapters::{
    adapt_dart_wasm, adapt_legacy_dylink, adapt_zig_wasm, bundled_grammar_candidate,
    bundled_grammar_candidates, parse_grammar_asset_manifest, verify_grammar_asset,
};

#[test]
fn selected_candidate_matches_full_manifest_without_promoting_unknown_language() {
    let full = bundled_grammar_candidates().expect("all bundled assets");
    for asset in &full.assets {
        let (selected, wasm) = bundled_grammar_candidate(&asset.language).expect("selected asset");
        assert_eq!(&selected, asset);
        assert_eq!(wasm.len(), asset.bytes);
    }
    assert!(bundled_grammar_candidate("unknown").is_err());
}

#[test]
fn erlang_source_asset_is_a_pinned_unqualified_candidate() {
    let (asset, wasm) = bundled_grammar_candidate("erlang").expect("Erlang candidate");
    assert_eq!(
        asset.grammar_commit,
        "836aa2b6c3af2c7cef3f84049b0ed6d44485a870"
    );
    assert_eq!(asset.abi_version, 14);
    assert_eq!(asset.release_status, "candidate_unvalidated");
    assert_eq!(wasm.len(), 421639);
}

#[test]
fn pascal_source_asset_is_a_pinned_unqualified_candidate() {
    let (asset, wasm) = bundled_grammar_candidate("pascal").expect("Pascal candidate");
    assert_eq!(
        asset.grammar_commit,
        "042119eca2e18a60e56317fb06ee3ba5c32cb447"
    );
    assert_eq!(asset.abi_version, 14);
    assert_eq!(asset.release_status, "candidate_unvalidated");
    assert_eq!(wasm.len(), 716886);
}
use serde_json::Value;

#[test]
fn published_schema_covers_every_current_candidate_and_field() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/grammar-asset-manifest.schema.json"
    ))
    .unwrap();
    let manifest: Value =
        serde_json::from_str(include_str!("../../../grammars/manifest.json")).unwrap();
    let assets = manifest["assets"].as_array().unwrap();
    assert!(assets.len() >= schema["properties"]["assets"]["minItems"].as_u64().unwrap() as usize);
    assert!(assets.len() <= schema["properties"]["assets"]["maxItems"].as_u64().unwrap() as usize);
    assert_eq!(
        schema["properties"]["build_tool"]["const"],
        manifest["build_tool"]
    );
    let root_fields = schema["properties"].as_object().unwrap();
    for name in manifest.as_object().unwrap().keys() {
        assert!(
            root_fields.contains_key(name),
            "schema missing root field {name}"
        );
    }
    let asset_fields = schema["$defs"]["asset"]["properties"].as_object().unwrap();
    let languages = schema["$defs"]["asset"]["properties"]["language"]["enum"]
        .as_array()
        .unwrap();
    for asset in assets {
        assert!(
            languages.contains(&asset["language"]),
            "schema missing {}",
            asset["language"]
        );
        for name in asset.as_object().unwrap().keys() {
            assert!(
                asset_fields.contains_key(name),
                "schema missing asset field {name}"
            );
        }
    }
    assert_eq!(
        schema["$defs"]["asset"]["properties"]["release_status"]["const"],
        "candidate_unvalidated"
    );
}

#[test]
fn bundled_candidates_pin_source_bytes_license_and_abi_without_claiming_support() {
    let manifest = bundled_grammar_candidates().expect("bundled manifest");
    assert_eq!(manifest.assets.len(), 32);
    for asset in &manifest.assets {
        let (wasm, license) = match asset.language.as_str() {
            "arkts" => (
                include_bytes!("../../../grammars/arkts/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/arkts/LICENSE").as_slice(),
            ),
            "c" => (
                include_bytes!("../../../grammars/c/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/c/LICENSE").as_slice(),
            ),
            "cfml" => (
                include_bytes!("../../../grammars/cfml/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/cfml/LICENSE").as_slice(),
            ),
            "cfquery" => (
                include_bytes!("../../../grammars/cfquery/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/cfquery/LICENSE").as_slice(),
            ),
            "cfscript" => (
                include_bytes!("../../../grammars/cfscript/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/cfscript/LICENSE").as_slice(),
            ),
            "cobol" => (
                include_bytes!("../../../grammars/cobol/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/cobol/LICENSE").as_slice(),
            ),
            "cpp" => (
                include_bytes!("../../../grammars/cpp/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/cpp/LICENSE").as_slice(),
            ),
            "csharp" => (
                include_bytes!("../../../grammars/csharp/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/csharp/LICENSE").as_slice(),
            ),
            "dart" => (
                include_bytes!("../../../grammars/dart/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/dart/LICENSE").as_slice(),
            ),
            "erlang" => (
                include_bytes!("../../../grammars/erlang/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/erlang/LICENSE").as_slice(),
            ),
            "pascal" => (
                include_bytes!("../../../grammars/pascal/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/pascal/LICENSE").as_slice(),
            ),
            "scala" => (
                include_bytes!("../../../grammars/scala/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/scala/LICENSE").as_slice(),
            ),
            "swift" => (
                include_bytes!("../../../grammars/swift/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/swift/LICENSE").as_slice(),
            ),
            "vbnet" => (
                include_bytes!("../../../grammars/vbnet/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/vbnet/LICENSE").as_slice(),
            ),
            "go" => (
                include_bytes!("../../../grammars/go/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/go/LICENSE").as_slice(),
            ),
            "javascript" => (
                include_bytes!("../../../grammars/javascript/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/javascript/LICENSE").as_slice(),
            ),
            "lua" => (
                include_bytes!("../../../grammars/lua/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/lua/LICENSE").as_slice(),
            ),
            "luau" => (
                include_bytes!("../../../grammars/luau/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/luau/LICENSE").as_slice(),
            ),
            "nix" => (
                include_bytes!("../../../grammars/nix/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/nix/LICENSE").as_slice(),
            ),
            "java" => (
                include_bytes!("../../../grammars/java/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/java/LICENSE").as_slice(),
            ),
            "kotlin" => (
                include_bytes!("../../../grammars/kotlin/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/kotlin/LICENSE").as_slice(),
            ),
            "python" => (
                include_bytes!("../../../grammars/python/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/python/LICENSE").as_slice(),
            ),
            "php" => (
                include_bytes!("../../../grammars/php/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/php/LICENSE").as_slice(),
            ),
            "r" => (
                include_bytes!("../../../grammars/r/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/r/LICENSE").as_slice(),
            ),
            "ruby" => (
                include_bytes!("../../../grammars/ruby/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/ruby/LICENSE").as_slice(),
            ),
            "typescript" => (
                include_bytes!("../../../grammars/typescript/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
            ),
            "tsx" => (
                include_bytes!("../../../grammars/tsx/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
            ),
            "zig" => (
                include_bytes!("../../../grammars/zig/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/zig/LICENSE").as_slice(),
            ),
            "objc" => (
                include_bytes!("../../../grammars/objc/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/objc/LICENSE").as_slice(),
            ),
            "solidity" => (
                include_bytes!("../../../grammars/solidity/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/solidity/LICENSE").as_slice(),
            ),
            "rust" => (
                include_bytes!("../../../grammars/rust/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/rust/LICENSE").as_slice(),
            ),
            "terraform" => (
                include_bytes!("../../../grammars/terraform/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/terraform/LICENSE").as_slice(),
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
fn dependency_dylink_adaptation_is_byte_pinned_and_rejects_changed_source() {
    for language in ["objc", "solidity"] {
        let (source, adapted) = if language == "objc" {
            (
                include_bytes!("../../../grammars/objc/source.wasm").as_slice(),
                include_bytes!("../../../grammars/objc/parser.wasm").as_slice(),
            )
        } else {
            (
                include_bytes!("../../../grammars/solidity/source.wasm").as_slice(),
                include_bytes!("../../../grammars/solidity/parser.wasm").as_slice(),
            )
        };
        assert_eq!(adapt_legacy_dylink(language, source).unwrap(), adapted);
        let mut changed = source.to_vec();
        changed[100] ^= 1;
        assert!(adapt_legacy_dylink(language, &changed).is_err());
    }
}

#[test]
fn zig_import_adaptation_is_byte_pinned_and_rejects_changed_source() {
    let source = include_bytes!("../../../grammars/zig/source.wasm");
    let adapted = include_bytes!("../../../grammars/zig/parser.wasm");
    assert_eq!(adapt_zig_wasm(source).unwrap(), adapted);
    let mut changed = source.to_vec();
    changed[200] ^= 1;
    assert!(adapt_zig_wasm(&changed).is_err());
}

#[test]
fn dart_rebuild_import_adaptation_is_byte_pinned_and_rejects_changed_source() {
    let source = include_bytes!("../../../grammars/dart/source.wasm");
    let adapted = include_bytes!("../../../grammars/dart/parser.wasm");
    assert_eq!(adapt_dart_wasm(source).unwrap(), adapted);
    let mut changed = source.to_vec();
    changed[200] ^= 1;
    assert!(adapt_dart_wasm(&changed).is_err());
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
    let unknown = original.replacen("\"language\": \"java\"", "\"language\": \"unknown\"", 1);
    assert!(parse_grammar_asset_manifest(unknown.as_bytes()).is_err());
    let wrong_symbol = original.replacen(
        "\"loader_symbol\": \"c_sharp\"",
        "\"loader_symbol\": \"csharp\"",
        1,
    );
    assert!(parse_grammar_asset_manifest(wrong_symbol.as_bytes()).is_err());
    let wrong_npm_integrity = original.replacen(
        "tree-sitter-arkts@0.2.0 sha512-j4KpZ21YdX5koXiuuslML24LoKLOzV6uZ1PMG//c/ynAoQfD7XY2LWnFxJiX0sOLqf2pOEYHdUmKFnXjm4QL4g==",
        "tree-sitter-arkts@0.2.0 sha512-forged",
        1,
    );
    assert!(parse_grammar_asset_manifest(wrong_npm_integrity.as_bytes()).is_err());
}

#[test]
fn changed_wasm_license_or_abi_never_qualifies_a_candidate() {
    let manifest = bundled_grammar_candidates().unwrap();
    let mut asset = manifest
        .assets
        .iter()
        .find(|asset| asset.language == "java")
        .expect("Java candidate")
        .clone();
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

    let python = manifest
        .assets
        .iter()
        .find(|asset| asset.language == "python")
        .expect("Python candidate");
    let python_wasm = include_bytes!("../../../grammars/python/parser.wasm");
    let python_license = include_bytes!("../../../grammars/python/LICENSE");
    assert!(verify_grammar_asset(python, python_wasm, python_license).is_ok());
    let mut altered_python = python_wasm.to_vec();
    altered_python[64] ^= 1;
    assert!(verify_grammar_asset(python, &altered_python, python_license).is_err());
    assert!(verify_grammar_asset(python, python_wasm, license).is_err());
}
