#![cfg(any(target_os = "macos", target_os = "linux"))]
use codeguard_cli::distribution_publication::{
    DistributionPublicationRequest, publish_signed_distribution_package,
};
use codeguard_cli::distribution_source::{DistributionTrustKey, DistributionVerificationContext};
use std::path::Path;
use std::sync::atomic::AtomicBool;
#[path = "support/signed_publication_fixture.rs"]
mod fixture;
use fixture::{cache, context, deadline, inputs, trust};
pub fn request<'a>(
    input: &'a (Vec<u8>, Vec<u8>, Vec<u8>),
    package: &'a [u8],
    cache: &'a Path,
    trust: &'a DistributionTrustKey,
    context: &'a DistributionVerificationContext<'a>,
) -> DistributionPublicationRequest<'a> {
    DistributionPublicationRequest {
        envelope_bytes: &input.0,
        manifest_bytes: &input.1,
        lock_bytes: &input.2,
        tool_id: "tool",
        package,
        cache_root: cache,
        trust,
        context,
    }
}
#[test]
fn signed_raw_and_archive_publish_at_original_lock_locators_and_reuse() {
    for (package, archive) in [
        (b"tool".as_slice(), false),
        (
            include_bytes!("../../../tests/fixtures/distribution/bundle-projection.zip").as_slice(),
            true,
        ),
    ] {
        let input = inputs(package, archive);
        let root = cache();
        let trust = trust();
        let context = context();
        let req = request(&input, package, &root, &trust, &context);
        let first =
            publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
                Some(200)
            })
            .unwrap();
        assert!(first.newly_published());
        assert_eq!(
            std::fs::read(root.join(first.origin_ref())).unwrap(),
            b"tool"
        );
        let second =
            publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
                Some(200)
            })
            .unwrap();
        assert!(!second.newly_published());
        assert_eq!(first.origin_ref(), second.origin_ref());
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn wrong_package_cancel_and_expired_final_clock_publish_nothing() {
    for mode in 0..3 {
        let input = inputs(b"tool", false);
        let root = cache();
        let trust = trust();
        let context = context();
        let req = request(
            &input,
            if mode == 0 { b"evil" } else { b"tool" },
            &root,
            &trust,
            &context,
        );
        let mut calls = 0;
        assert!(
            publish_signed_distribution_package(
                &req,
                deadline(),
                &AtomicBool::new(mode == 1),
                || {
                    calls += 1;
                    Some(if mode == 2 && calls > 1 { 500 } else { 200 })
                }
            )
            .is_err()
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn failure_after_publication_does_not_claim_success_and_can_be_revalidated() {
    let input = inputs(b"tool", false);
    let root = cache();
    let trust = trust();
    let context = context();
    let req = request(&input, b"tool", &root, &trust, &context);
    let mut calls = 0;
    assert!(
        publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
            calls += 1;
            if calls >= 3 { None } else { Some(200) }
        })
        .is_err()
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let retried =
        publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
            Some(200)
        })
        .unwrap();
    assert!(!retried.newly_published());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn bad_signature_missing_tool_foreign_platform_and_clock_rollback_preserve_empty_cache() {
    for mode in 0..4 {
        let mut input = inputs(b"tool", false);
        if mode == 0 {
            let mut e: serde_json::Value = serde_json::from_slice(&input.0).unwrap();
            e["signature_hex"] = "0".repeat(128).into();
            input.0 = serde_json::to_vec(&e).unwrap();
        }
        let root = cache();
        let trust = trust();
        let mut context = context();
        if mode == 2 {
            context.platform = "foreign_platform";
        }
        let mut req = request(&input, b"tool", &root, &trust, &context);
        if mode == 1 {
            req.tool_id = "other";
        }
        let mut calls = 0;
        let error =
            publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
                calls += 1;
                Some(if mode == 3 && calls > 1 { 199 } else { 200 })
            })
            .unwrap_err();
        assert_eq!(
            error,
            match mode {
                0 => "distribution_signature_invalid",
                1 => "distribution_publication_artifact_missing",
                2 => "distribution_publication_platform_mismatch",
                _ => "distribution_publication_clock_rollback",
            }
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn corrupt_existing_destination_is_preserved_and_not_replaced() {
    let input = inputs(b"tool", false);
    let root = cache();
    let trust = trust();
    let context = context();
    let req = request(&input, b"tool", &root, &trust, &context);
    let first =
        publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || {
            Some(200)
        })
        .unwrap();
    let path = root.join(first.origin_ref());
    std::fs::write(&path, b"evil").unwrap();
    assert!(
        publish_signed_distribution_package(&req, deadline(), &AtomicBool::new(false), || Some(
            200
        ))
        .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), b"evil");
    std::fs::remove_dir_all(root).unwrap();
}
