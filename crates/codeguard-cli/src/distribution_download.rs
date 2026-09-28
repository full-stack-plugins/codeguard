//! 签名、网络范围、有界 HTTPS 与制品发布的共同编排。
use crate::distribution_bundle::digest_bytes;
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
use crate::distribution_publication::{
    DistributionPublicationRequest, PublishedDistributionArtifact,
    publish_signed_distribution_package,
};
use crate::distribution_source::verify_signed_distribution;
pub use crate::signed_distribution_download_request::SignedDistributionDownloadRequest;
use crate::tool_identity::current_platform_id;
use codeguard_runtime::{
    PackageDownloadRequest, download_verified_package, package_download_authority,
    validate_package_download_request,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 明确调用签名下载及缓存发布，缺来源/站点许可时请求前拒绝。
/// 参数含冻结宿主输入、共享预算/取消及可信时钟；返回制品发布收据。
/// 不安装外部运行时、执行制品或签发质量结论；宿主输入真实性须独立保证。
pub async fn download_and_publish_signed_distribution(
    request: &SignedDistributionDownloadRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
    mut clock: impl FnMut() -> Option<u64>,
) -> Result<PublishedDistributionArtifact, &'static str> {
    budget(deadline, cancelled)?;
    if current_platform_id() != Some(request.context.platform) {
        return Err("distribution_download_platform_mismatch");
    }
    let now = clock().ok_or("distribution_clock_unavailable")?;
    if request
        .context
        .now_unix
        .is_none_or(|previous| now < previous)
    {
        return Err("distribution_download_clock_invalid");
    }
    let mut context = *request.context;
    context.now_unix = Some(now);
    verify_signed_distribution(
        request.envelope_bytes,
        request.manifest_bytes,
        request.lock_bytes,
        request.trust,
        &context,
    )?;
    budget(deadline, cancelled)?;
    let manifest = parse_distribution_manifest(request.manifest_bytes)?;
    if manifest.schema_version != "1.2" {
        return Err("distribution_download_mapping_required");
    }
    let bound = bind_distribution_manifest(&manifest, request.lock_bytes)?;
    let artifact = bound
        .into_iter()
        .find(|a| a.tool_id == request.tool_id && a.platform == context.platform)
        .ok_or("distribution_download_artifact_missing")?;
    let download = PackageDownloadRequest {
        url: &artifact.download_url,
        expected_size: artifact.package_size_bytes,
        expected_sha256: digest_bytes(&artifact.package_sha256)?,
        redirect_authorities: request.network_authorities,
    };
    validate_package_download_request(&download)?;
    let initial = package_download_authority(download.url)?;
    if !request.network_authorities.contains(&initial.as_str()) {
        return Err("distribution_download_network_unapproved");
    }
    budget(deadline, cancelled)?;
    let package = download_verified_package(&download, deadline, cancelled).await?;
    // 下载后的发布入口再次核验时间、签名、原始身份与全部包内容；预算不重置。
    publish_signed_distribution_package(
        &DistributionPublicationRequest {
            envelope_bytes: request.envelope_bytes,
            manifest_bytes: request.manifest_bytes,
            lock_bytes: request.lock_bytes,
            tool_id: request.tool_id,
            package: &package,
            cache_root: request.cache_root,
            trust: request.trust,
            context: &context,
        },
        deadline,
        cancelled,
        clock,
    )
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("distribution_download_cancelled")
    } else if Instant::now() >= deadline {
        Err("distribution_download_deadline_exceeded")
    } else {
        Ok(())
    }
}
