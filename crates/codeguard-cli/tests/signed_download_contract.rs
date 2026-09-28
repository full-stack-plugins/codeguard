#![cfg(any(target_os = "macos", target_os = "linux"))]
use codeguard_cli::distribution_download::{
    SignedDistributionDownloadRequest, download_and_publish_signed_distribution,
};
use std::sync::atomic::AtomicBool;
#[path = "support/signed_publication_fixture.rs"]
mod fixture;
#[test]
fn invalid_source_scope_and_network_permission_fail_before_download() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for mode in 0..5 {
        let mut inputs = fixture::inputs(b"tool", false);
        let root = fixture::cache();
        let trust = fixture::trust();
        let mut context = fixture::context();
        if mode == 0 {
            let mut e: serde_json::Value = serde_json::from_slice(&inputs.0).unwrap();
            e["signature_hex"] = "0".repeat(128).into();
            inputs.0 = serde_json::to_vec(&e).unwrap();
        }
        if mode == 1 {
            context.platform = "foreign_platform";
        }
        let request = SignedDistributionDownloadRequest {
            envelope_bytes: &inputs.0,
            manifest_bytes: &inputs.1,
            lock_bytes: &inputs.2,
            tool_id: if mode == 2 { "other" } else { "tool" },
            cache_root: &root,
            trust: &trust,
            context: &context,
            network_authorities: if mode == 4 { &["*.org:443"] } else { &[] },
        };
        let error = runtime
            .block_on(download_and_publish_signed_distribution(
                &request,
                fixture::deadline(),
                &AtomicBool::new(false),
                || Some(200),
            ))
            .unwrap_err();
        assert_eq!(
            error,
            match mode {
                0 => "distribution_signature_invalid",
                1 => "distribution_download_platform_mismatch",
                2 => "distribution_download_artifact_missing",
                3 => "distribution_download_network_unapproved",
                _ => "package_download_redirect_scope_invalid",
            }
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
#[ignore = "需显式 CODEGUARD_TEST_PACKAGE_URL 和 CODEGUARD_TEST_PACKAGE_FILE，使用真实公共 TLS 下载测试包"]
fn real_public_https_signed_download_and_temporary_publication() {
    let url = std::env::var("CODEGUARD_TEST_PACKAGE_URL").unwrap();
    let file = std::env::var("CODEGUARD_TEST_PACKAGE_FILE").unwrap();
    let bytes =
        codeguard_runtime::read_bounded_regular_file(std::path::Path::new(&file), 1024 * 1024)
            .unwrap();
    let initial = codeguard_runtime::package_download_authority(&url).unwrap();
    let allowed = [initial.as_str()];
    let inputs = fixture::inputs_at_url(&bytes, false, &url);
    let root = fixture::cache();
    let trust = fixture::trust();
    let context = fixture::context();
    let request = SignedDistributionDownloadRequest {
        envelope_bytes: &inputs.0,
        manifest_bytes: &inputs.1,
        lock_bytes: &inputs.2,
        tool_id: "tool",
        cache_root: &root,
        trust: &trust,
        context: &context,
        network_authorities: &allowed,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(download_and_publish_signed_distribution(
        &request,
        std::time::Instant::now() + std::time::Duration::from_secs(20),
        &AtomicBool::new(false),
        || Some(200),
    ));
    if let Ok(ref published) = result {
        assert_eq!(
            std::fs::read(root.join(published.origin_ref())).unwrap(),
            bytes
        );
    }
    std::fs::remove_dir_all(root).unwrap();
    assert!(result.unwrap().newly_published());
}
