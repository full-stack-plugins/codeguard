//! 发行声明与冻结归档的内容关联；不提供下载、批准或文件发布。
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
use codeguard_runtime::{
    ArchiveUnpackRequest, ProjectedBundle, project_unpacked_bundle, unpack_package_archive_tree,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 核对清单原字节、工具锁、原包、入口与显式 bundle 根的精确内容身份。
/// 参数为清单/锁、工具与平台、冻结包和共享预算；返回只读子树及未消费成员。
/// 匹配成功仅为内容关联，不证明清单可信、安装完成或原生工具可运行。
#[allow(clippy::too_many_arguments)]
pub fn inspect_distribution_bundle(
    manifest_bytes: &[u8],
    lock_bytes: &[u8],
    tool_id: &str,
    platform: &str,
    package: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<ProjectedBundle, &'static str> {
    budget(deadline, cancelled)?;
    let manifest = parse_distribution_manifest(manifest_bytes)?;
    let artifacts = bind_distribution_manifest(&manifest, lock_bytes)?;
    budget(deadline, cancelled)?;
    let artifact = artifacts
        .into_iter()
        .find(|a| a.tool_id == tool_id && a.platform == platform)
        .ok_or("distribution_bundle_artifact_missing")?;
    let root = artifact
        .bundle_archive_root
        .as_deref()
        .ok_or("distribution_bundle_mapping_missing")?;
    let expected = artifact
        .bundle_tree_sha256
        .as_deref()
        .ok_or("distribution_bundle_mapping_missing")?;
    let entry = artifact
        .entrypoint
        .as_deref()
        .ok_or("distribution_bundle_entrypoint_missing")?;
    if package.len() as u64 != artifact.package_size_bytes {
        return Err("distribution_package_size_mismatch");
    }
    let tree = unpack_package_archive_tree(
        package,
        &ArchiveUnpackRequest {
            format: &artifact.format,
            entrypoint: entry,
            package_sha256: digest_bytes(&artifact.package_sha256)?,
            entrypoint_sha256: digest_bytes(&artifact.binary_sha256)?,
            max_unpacked_bytes: artifact.unpacked_size_limit_bytes,
        },
        deadline,
        cancelled,
    )?;
    project_unpacked_bundle(tree, root, expected, deadline, cancelled)
}
pub(crate) fn digest_bytes(value: &str) -> Result<[u8; 32], &'static str> {
    let mut result = [0; 32];
    if value.len() != 64 {
        return Err("distribution_digest_invalid");
    }
    for (index, slot) in result.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "distribution_digest_invalid")?;
    }
    Ok(result)
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("distribution_bundle_cancelled")
    } else if Instant::now() >= deadline {
        Err("distribution_bundle_deadline_exceeded")
    } else {
        Ok(())
    }
}
