use codeguard_cli::distribution_source::{
    DistributionTrustKey, DistributionVerificationContext, verify_signed_distribution,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn lock() -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":"1.0","lock_id":"test","tools":[{"id":"ruff","version":"1","platform":"linux_x86_64","binary_sha256":"a".repeat(64),"adapter":{"id":"ruff","version":"1"},"rule_source":{"kind":"native_builtin","id":"ruff","sha256":"b".repeat(64)},"origin":{"kind":"managed_cache","ref":format!("{}.bin","a".repeat(64))}}]})).unwrap()
}
fn manifest() -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":"1.2","manifest_id":"release-test","lock_sha256":sha(&lock()),"artifacts":[{"tool_id":"ruff","version":"1","platform":"linux_x86_64","binary_sha256":"a".repeat(64),"origin_ref_sha256":sha(format!("{}.bin","a".repeat(64)).as_bytes()),"download_url":"https://release.example.org/tool","package_sha256":"a".repeat(64),"package_size_bytes":4,"format":"raw","entrypoint":null,"unpacked_size_limit_bytes":4,"bundle_tree_sha256":null,"bundle_archive_root":null,"install_tree_sha256":null}]})).unwrap()
}
fn keypair() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[19; 32]).unwrap()
}
fn trust() -> DistributionTrustKey {
    DistributionTrustKey {
        key_id: "release-key".into(),
        publisher_id: "publisher".into(),
        public_key: keypair().public_key().as_ref().try_into().unwrap(),
        valid_from: 100,
        valid_until: 1000,
        revoked: false,
    }
}
fn context() -> DistributionVerificationContext<'static> {
    DistributionVerificationContext {
        publisher_id: "publisher",
        channel: "stable",
        platform: "linux_x86_64",
        now_unix: Some(200),
        minimum_sequence: 2,
        max_lifetime_seconds: 400,
    }
}
fn payload() -> Value {
    json!({"schema_version":"1.0","publisher_id":"publisher","channel":"stable","platform":"linux_x86_64","release_sequence":2,"issued_at":150,"expires_at":500,"manifest_sha256":sha(&manifest()),"lock_sha256":sha(&lock())})
}
fn envelope(value: &Value, domain: &[u8]) -> Vec<u8> {
    let text = serde_json::to_string(value).unwrap();
    let mut message = domain.to_vec();
    message.extend_from_slice(b"release-key\0");
    message.extend_from_slice(text.as_bytes());
    let sig = keypair().sign(&message);
    let signature = sig
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    serde_json::to_vec(&json!({"schema_version":"1.0","key_id":"release-key","release_json":text,"signature_hex":signature})).unwrap()
}
#[test]
fn signature_binds_original_manifest_lock_and_target_without_install_authority() {
    let bound = verify_signed_distribution(
        &envelope(&payload(), b"codeguard.distribution.v1\0"),
        &manifest(),
        &lock(),
        &trust(),
        &context(),
    )
    .unwrap();
    assert_eq!(bound.manifest_sha256(), sha(&manifest()));
    assert_eq!(bound.lock_sha256(), sha(&lock()));
    assert_eq!(bound.expires_at(), 500);
    assert_eq!(bound.platform(), "linux_x86_64");
    assert_eq!(bound.release_sequence(), 2);
}
#[test]
fn wrong_scope_rollback_time_and_expiration_are_rejected() {
    for (field, value, expected) in [
        (
            "publisher_id",
            json!("other"),
            "distribution_scope_mismatch",
        ),
        ("channel", json!("nightly"), "distribution_scope_mismatch"),
        (
            "platform",
            json!("macos_arm64"),
            "distribution_scope_mismatch",
        ),
        (
            "release_sequence",
            json!(1),
            "distribution_revision_rollback",
        ),
        ("issued_at", json!(201), "distribution_lifetime_invalid"),
        ("expires_at", json!(200), "distribution_lifetime_invalid"),
        ("expires_at", json!(999), "distribution_lifetime_invalid"),
    ] {
        let mut p = payload();
        p[field] = value;
        assert_eq!(
            verify_signed_distribution(
                &envelope(&p, b"codeguard.distribution.v1\0"),
                &manifest(),
                &lock(),
                &trust(),
                &context()
            )
            .unwrap_err(),
            expected
        );
    }
}
#[test]
fn changed_bytes_and_other_protocol_signatures_are_not_distribution_approval() {
    let signed = envelope(&payload(), b"codeguard.distribution.v1\0");
    let mut m = manifest();
    m.push(b' ');
    assert_eq!(
        verify_signed_distribution(&signed, &m, &lock(), &trust(), &context()).unwrap_err(),
        "distribution_manifest_digest_mismatch"
    );
    let mut l = lock();
    l.push(b' ');
    assert_eq!(
        verify_signed_distribution(&signed, &manifest(), &l, &trust(), &context()).unwrap_err(),
        "distribution_signed_lock_mismatch"
    );
    assert_eq!(
        verify_signed_distribution(
            &envelope(&payload(), b"codeguard.approval.v1\0"),
            &manifest(),
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_signature_invalid"
    );
}
#[test]
fn revoked_wrong_key_or_missing_host_clock_cannot_bind() {
    let signed = envelope(&payload(), b"codeguard.distribution.v1\0");
    let mut t = trust();
    t.revoked = true;
    assert_eq!(
        verify_signed_distribution(&signed, &manifest(), &lock(), &t, &context()).unwrap_err(),
        "distribution_key_revoked"
    );
    let mut t = trust();
    t.public_key = [1; 32];
    assert_eq!(
        verify_signed_distribution(&signed, &manifest(), &lock(), &t, &context()).unwrap_err(),
        "distribution_signature_invalid"
    );
    let mut c = context();
    c.now_unix = None;
    assert_eq!(
        verify_signed_distribution(&signed, &manifest(), &lock(), &trust(), &c).unwrap_err(),
        "distribution_clock_unavailable"
    );
}
#[test]
fn signed_malformed_manifest_is_rejected_even_with_matching_hash() {
    let mut m: Value = serde_json::from_slice(&manifest()).unwrap();
    m["approved"] = true.into();
    let m = serde_json::to_vec(&m).unwrap();
    let mut p = payload();
    p["manifest_sha256"] = sha(&m).into();
    assert_eq!(
        verify_signed_distribution(
            &envelope(&p, b"codeguard.distribution.v1\0"),
            &m,
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_manifest_invalid"
    );
}
#[test]
fn malformed_envelopes_payloads_and_oversized_inputs_are_rejected() {
    let signed = envelope(&payload(), b"codeguard.distribution.v1\0");
    for mutate in [0, 1, 2] {
        let mut e: Value = serde_json::from_slice(&signed).unwrap();
        let expected = match mutate {
            0 => {
                e["approved"] = true.into();
                "distribution_envelope_invalid"
            }
            1 => {
                e["release_json"] = format!("{} ", e["release_json"].as_str().unwrap()).into();
                "distribution_signature_invalid"
            }
            _ => {
                e["signature_hex"] = "A".repeat(128).into();
                "distribution_signature_invalid"
            }
        };
        assert_eq!(
            verify_signed_distribution(
                &serde_json::to_vec(&e).unwrap(),
                &manifest(),
                &lock(),
                &trust(),
                &context()
            )
            .unwrap_err(),
            expected
        );
    }
    let mut p = payload();
    p["approved"] = true.into();
    assert_eq!(
        verify_signed_distribution(
            &envelope(&p, b"codeguard.distribution.v1\0"),
            &manifest(),
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_payload_invalid"
    );
    let duplicated =
        String::from_utf8(signed.clone())
            .unwrap()
            .replacen('{', "{\"schema_version\":\"1.0\",", 1);
    assert_eq!(
        verify_signed_distribution(
            duplicated.as_bytes(),
            &manifest(),
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_envelope_invalid"
    );
    assert_eq!(
        verify_signed_distribution(
            &vec![b' '; 16385],
            &manifest(),
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_signature_input_limit"
    );
}
#[test]
fn signed_manifest_must_still_match_original_lock_and_contain_selected_platform() {
    let mut m: Value = serde_json::from_slice(&manifest()).unwrap();
    m["artifacts"][0]["binary_sha256"] = "c".repeat(64).into();
    m["artifacts"][0]["package_sha256"] = "c".repeat(64).into();
    let m = serde_json::to_vec(&m).unwrap();
    let mut p = payload();
    p["manifest_sha256"] = sha(&m).into();
    assert_eq!(
        verify_signed_distribution(
            &envelope(&p, b"codeguard.distribution.v1\0"),
            &m,
            &lock(),
            &trust(),
            &context()
        )
        .unwrap_err(),
        "distribution_tool_identity_mismatch"
    );
    let mut p = payload();
    p["platform"] = "macos_arm64".into();
    let mut c = context();
    c.platform = "macos_arm64";
    assert_eq!(
        verify_signed_distribution(
            &envelope(&p, b"codeguard.distribution.v1\0"),
            &manifest(),
            &lock(),
            &trust(),
            &c
        )
        .unwrap_err(),
        "distribution_platform_unbound"
    );
}
#[test]
fn host_key_lifetime_publisher_and_context_are_independent_constraints() {
    let signed = envelope(&payload(), b"codeguard.distribution.v1\0");
    for mutate in [0, 1, 2] {
        let mut t = trust();
        let expected = match mutate {
            0 => {
                t.publisher_id = "other".into();
                "distribution_trust_invalid"
            }
            1 => {
                t.valid_until = 200;
                "distribution_key_not_current"
            }
            _ => {
                t.valid_from = 201;
                "distribution_key_not_current"
            }
        };
        assert_eq!(
            verify_signed_distribution(&signed, &manifest(), &lock(), &t, &context()).unwrap_err(),
            expected
        );
    }
    let mut c = context();
    c.minimum_sequence = 0;
    assert_eq!(
        verify_signed_distribution(&signed, &manifest(), &lock(), &trust(), &c).unwrap_err(),
        "distribution_context_invalid"
    );
}
