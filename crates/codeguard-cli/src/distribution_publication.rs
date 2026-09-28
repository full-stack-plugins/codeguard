//! 明确调用的签名发行包发布编排；不开放 CLI 未批准 apply 或隐式联网。
use crate::distribution_layout::inspect_distribution_layout;
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
pub use crate::distribution_publication_request::DistributionPublicationRequest;
use crate::distribution_raw::inspect_distribution_raw;
use crate::distribution_source::verify_signed_distribution;
pub use crate::published_distribution_artifact::PublishedDistributionArtifact;
use crate::tool_identity::current_platform_id;
use codeguard_runtime::{publish_tool_bundle, publish_tool_bytes};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 核验签名、原锁及包内容后发布选定工具，失败不覆盖既有内容。
/// 参数包含冻结包与宿主上下文、共享预算/取消，以及独立可信时钟回调。
/// 返回仅为制品发布；网络、运行时、原生可运行性和宿主授权须独立完成。
/// 发布后时钟/预算失败可能留下完整制品，但不返回成功，重试必须重新核验。
pub fn publish_signed_distribution_package(
    request: &DistributionPublicationRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
    mut clock: impl FnMut() -> Option<u64>,
) -> Result<PublishedDistributionArtifact, &'static str> {
    budget(deadline, cancelled)?;
    if current_platform_id() != Some(request.context.platform) {
        return Err("distribution_publication_platform_mismatch");
    }
    let mut previous = request
        .context
        .now_unix
        .ok_or("distribution_clock_unavailable")?;
    verify_current(request, &mut previous, &mut clock)?;
    budget(deadline, cancelled)?;
    let manifest = parse_distribution_manifest(request.manifest_bytes)?;
    if manifest.schema_version != "1.2" {
        return Err("distribution_publication_mapping_required");
    }
    let bound = bind_distribution_manifest(&manifest, request.lock_bytes)?;
    let artifact = bound
        .into_iter()
        .find(|a| a.tool_id == request.tool_id && a.platform == request.context.platform)
        .ok_or("distribution_publication_artifact_missing")?;
    let result = if artifact.format == "raw" {
        let raw = inspect_distribution_raw(
            request.manifest_bytes,
            request.lock_bytes,
            request.tool_id,
            request.context.platform,
            request.package,
            deadline,
            cancelled,
        )?;
        // 内容核验之后、实际发布之前再次核对可信时间与签名期限。
        verify_current(request, &mut previous, &mut clock)?;
        budget(deadline, cancelled)?;
        let installed = publish_tool_bytes(
            request.cache_root,
            raw.bytes(),
            raw.binary_sha256(),
            deadline,
            cancelled,
        )?;
        if installed.relative_path != raw.origin_ref() {
            return Err("distribution_publication_locator_mismatch");
        }
        PublishedDistributionArtifact {
            origin_ref: installed.relative_path,
            newly_published: installed.newly_published,
        }
    } else {
        let layout = inspect_distribution_layout(
            request.manifest_bytes,
            request.lock_bytes,
            request.tool_id,
            request.context.platform,
            request.package,
            deadline,
            cancelled,
        )?;
        verify_current(request, &mut previous, &mut clock)?;
        budget(deadline, cancelled)?;
        let installed = publish_tool_bundle(
            request.cache_root,
            layout.tree(),
            layout.install_tree_sha256(),
            deadline,
            cancelled,
        )?;
        let expected = format!(
            "{}/{}",
            installed.relative_path,
            artifact
                .entrypoint
                .as_deref()
                .ok_or("distribution_layout_entrypoint_missing")?
        );
        if expected != layout.origin_ref() {
            return Err("distribution_publication_locator_mismatch");
        }
        PublishedDistributionArtifact {
            origin_ref: expected,
            newly_published: installed.newly_published,
        }
    };
    // 发布后的期限/取消失败保留完整内容供重新核验，不能声称本轮成功。
    verify_current(request, &mut previous, &mut clock)?;
    budget(deadline, cancelled)?;
    Ok(result)
}
fn verify_current(
    request: &DistributionPublicationRequest<'_>,
    previous: &mut u64,
    clock: &mut impl FnMut() -> Option<u64>,
) -> Result<(), &'static str> {
    let now = clock().ok_or("distribution_clock_unavailable")?;
    if now < *previous {
        return Err("distribution_publication_clock_rollback");
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
    *previous = now;
    Ok(())
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("distribution_publication_cancelled")
    } else if Instant::now() >= deadline {
        Err("distribution_publication_deadline_exceeded")
    } else {
        Ok(())
    }
}
