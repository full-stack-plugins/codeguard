use codeguard_cli::distribution_manifest::{
    bind_distribution_manifest, parse_distribution_manifest,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn lock() -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":"1.0","lock_id":"candidate","tools":[{"id":"ruff","version":"1","platform":"linux_x86_64","binary_sha256":"a".repeat(64),"adapter":{"id":"ruff","version":"1"},"rule_source":{"kind":"native_builtin","id":"ruff","sha256":"b".repeat(64)},"origin":{"kind":"managed_cache","ref":"ruff/tool"}}]})).unwrap()
}
fn manifest() -> Value {
    json!({"schema_version":"1.0","manifest_id":"test","lock_sha256":format!("{:x}",Sha256::digest(lock())),"artifacts":[{"tool_id":"ruff","version":"1","platform":"linux_x86_64","binary_sha256":"a".repeat(64),"origin_ref_sha256":format!("{:x}",Sha256::digest(b"ruff/tool")),"download_url":"https://releases.example.org/ruff.bin","package_sha256":"a".repeat(64),"package_size_bytes":100,"format":"raw","entrypoint":null,"unpacked_size_limit_bytes":100,"bundle_tree_sha256":null}]})
}
fn parse(
    v: &Value,
) -> Result<codeguard_cli::distribution_manifest::DistributionManifest, &'static str> {
    parse_distribution_manifest(&serde_json::to_vec(v).unwrap())
}
#[test]
fn valid_manifest_binds_exact_candidate_without_approval() {
    let m = parse(&manifest()).unwrap();
    assert_eq!(bind_distribution_manifest(&m, &lock()).unwrap().len(), 1);
}
#[test]
fn changed_lock_or_entry_identity_cannot_bind() {
    for field in ["tool_id", "version", "binary_sha256", "origin_ref_sha256"] {
        let mut m = manifest();
        m["artifacts"][0][field] = if field.ends_with("sha256") {
            "c".repeat(64)
        } else {
            "other".into()
        }
        .into();
        if field == "binary_sha256" {
            m["artifacts"][0]["package_sha256"] = m["artifacts"][0][field].clone();
        }
        assert!(bind_distribution_manifest(&parse(&m).unwrap(), &lock()).is_err());
    }
}
#[test]
fn unknown_approval_and_duplicate_rows_are_rejected() {
    let mut m = manifest();
    m["approved"] = true.into();
    assert!(parse(&m).is_err());
    let mut m = manifest();
    let row = m["artifacts"][0].clone();
    m["artifacts"].as_array_mut().unwrap().push(row);
    assert!(parse(&m).is_err());
}
#[test]
fn unsafe_or_credential_urls_are_rejected() {
    for url in [
        "http://example.org/tool",
        "https://user:password@example.org/tool",
        "https://example.org/tool?token=secret",
        "https://example.org/tool#fragment",
        "https://example.org/\nheader",
    ] {
        let mut m = manifest();
        m["artifacts"][0]["download_url"] = url.into();
        assert!(parse(&m).is_err(), "{url:?}");
    }
}
#[test]
fn archive_entrypoint_and_size_are_bounded() {
    let mut m = manifest();
    m["artifacts"][0]["format"] = "zip".into();
    m["artifacts"][0]["entrypoint"] = "bin/ruff".into();
    assert!(parse(&m).is_ok());
    for path in [
        "../tool",
        "/absolute",
        "C:\\tool",
        "bin/../tool",
        "bin//tool",
    ] {
        m["artifacts"][0]["entrypoint"] = path.into();
        assert!(parse(&m).is_err());
    }
}
#[test]
fn zero_digest_and_excessive_size_cannot_be_placeholders() {
    for (field, value) in [
        ("package_sha256", Value::String("0".repeat(64))),
        ("package_size_bytes", json!(0)),
        ("unpacked_size_limit_bytes", json!(536870913)),
    ] {
        let mut m = manifest();
        m["artifacts"][0][field] = value;
        assert!(parse(&m).is_err());
    }
}

#[test]
fn binding_revalidates_mutated_structs_and_original_lock_bytes() {
    let mut parsed = parse(&manifest()).unwrap();
    parsed.artifacts[0].download_url = "http://example.org/tool".into();
    assert!(bind_distribution_manifest(&parsed, &lock()).is_err());
    let mut changed = lock();
    changed.push(b' ');
    assert_eq!(
        bind_distribution_manifest(&parse(&manifest()).unwrap(), &changed).unwrap_err(),
        "distribution_lock_digest_mismatch"
    );
}

#[test]
fn duplicate_json_fields_are_not_silently_replaced() {
    let value = serde_json::to_string(&manifest()).unwrap();
    let duplicate = value.replacen(
        "\"manifest_id\":\"test\"",
        "\"manifest_id\":\"first\",\"manifest_id\":\"test\"",
        1,
    );
    assert!(parse_distribution_manifest(duplicate.as_bytes()).is_err());
}

#[test]
fn raw_packages_cannot_claim_archive_metadata_or_different_bytes() {
    for (field, value) in [
        ("entrypoint", json!("bin/ruff")),
        ("bundle_tree_sha256", json!("c".repeat(64))),
        ("package_sha256", json!("c".repeat(64))),
        ("unpacked_size_limit_bytes", json!(101)),
    ] {
        let mut m = manifest();
        m["artifacts"][0][field] = value;
        assert!(parse(&m).is_err(), "{field}");
    }
}

#[test]
fn supported_archives_bind_and_invalid_versions_or_paths_fail() {
    for format in ["zip", "tar_gz"] {
        let mut m = manifest();
        m["artifacts"][0]["format"] = format.into();
        m["artifacts"][0]["entrypoint"] = "bin/ruff".into();
        assert_eq!(
            bind_distribution_manifest(&parse(&m).unwrap(), &lock())
                .unwrap()
                .len(),
            1
        );
        for path in ["bin/", "bin/./ruff", "bin/\u{7f}ruff"] {
            m["artifacts"][0]["entrypoint"] = path.into();
            assert!(parse(&m).is_err());
        }
    }
    let mut m = manifest();
    m["artifacts"][0]["version"] = "1\u{85}".into();
    assert!(parse(&m).is_err());
    let mut m = manifest();
    m["artifacts"][0]["download_url"] = "https://example.org".into();
    assert!(parse(&m).is_ok());
}

#[test]
fn oversized_and_empty_manifests_are_rejected_before_binding() {
    assert_eq!(
        parse_distribution_manifest(&vec![b' '; 256 * 1024 + 1]).unwrap_err(),
        "distribution_manifest_too_large"
    );
    let mut m = manifest();
    m["artifacts"] = json!([]);
    assert!(parse(&m).is_err());
}

#[test]
fn versioned_bundle_mapping_requires_explicit_root_and_tree_identity() {
    let mut m = manifest();
    m["schema_version"] = "1.1".into();
    m["artifacts"][0]["format"] = "tar_gz".into();
    m["artifacts"][0]["entrypoint"] = "release/bin/tool".into();
    m["artifacts"][0]["bundle_tree_sha256"] = "c".repeat(64).into();
    m["artifacts"][0]["bundle_archive_root"] = "release/libexec".into();
    assert!(parse(&m).is_ok());
    m["artifacts"][0]["bundle_archive_root"] = "".into();
    assert!(parse(&m).is_ok());
    for root in [
        "../release",
        "/release",
        "release/",
        "release//lib",
        "release/./lib",
        "C:\\lib",
    ] {
        m["artifacts"][0]["bundle_archive_root"] = root.into();
        assert!(parse(&m).is_err(), "{root}");
    }
    m["artifacts"][0]["bundle_archive_root"] = Value::Null;
    assert!(parse(&m).is_err());
    m["artifacts"][0]["bundle_archive_root"] = "release/libexec".into();
    m["artifacts"][0]["bundle_tree_sha256"] = Value::Null;
    assert!(parse(&m).is_err());
}

#[test]
fn old_version_and_raw_packages_cannot_claim_bundle_root_mapping() {
    let mut m = manifest();
    m["artifacts"][0]["bundle_archive_root"] = "root".into();
    assert!(parse(&m).is_err());
    m["schema_version"] = "1.1".into();
    assert!(parse(&m).is_err());
    m["artifacts"][0]["bundle_archive_root"] = Value::Null;
    assert!(parse(&m).is_ok());
}

#[test]
fn complete_layout_digest_is_versioned_and_required_for_archives_only() {
    let mut m = manifest();
    m["schema_version"] = "1.2".into();
    assert!(parse(&m).is_ok());
    m["artifacts"][0]["install_tree_sha256"] = "c".repeat(64).into();
    assert!(parse(&m).is_err());
    m["artifacts"][0]["format"] = "zip".into();
    m["artifacts"][0]["entrypoint"] = "release/bin/tool".into();
    assert!(parse(&m).is_ok());
    for invalid in [Value::Null, json!("0".repeat(64)), json!("invalid")] {
        m["artifacts"][0]["install_tree_sha256"] = invalid;
        assert!(parse(&m).is_err());
    }
    m["artifacts"][0]["install_tree_sha256"] = "c".repeat(64).into();
    for version in ["1.0", "1.1"] {
        m["schema_version"] = version.into();
        assert!(parse(&m).is_err());
    }
}
