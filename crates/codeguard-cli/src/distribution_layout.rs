//! 完整发行包布局与缓存锁路径的显式内容绑定。
use crate::distribution_bundle::digest_bytes;
use crate::distribution_layout_result::DistributionLayout;
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
use crate::tool_lock::parse_tool_lock_document;
use codeguard_runtime::{
    ArchiveUnpackRequest, project_unpacked_bundle, unpack_package_archive_tree,
    verify_unpacked_bundle_tree,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 验证完整包内容与内容寻址安装目录的入口/bundle 锁路径。
/// 参数为清单/锁原字节、工具/平台、冻结包及共享预算；返回只读布局。
/// 不写入或重写锁，不下载、执行或授予来源批准。
pub fn inspect_distribution_layout(
    manifest_bytes: &[u8],
    lock_bytes: &[u8],
    tool_id: &str,
    platform: &str,
    package: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<DistributionLayout, &'static str> {
    budget(deadline, cancelled)?;
    let manifest = parse_distribution_manifest(manifest_bytes)?;
    let bound = bind_distribution_manifest(&manifest, lock_bytes)?;
    let artifact = bound
        .into_iter()
        .find(|a| a.tool_id == tool_id && a.platform == platform)
        .ok_or("distribution_layout_artifact_missing")?;
    let expected = artifact
        .install_tree_sha256
        .as_deref()
        .ok_or("distribution_install_layout_missing")?;
    let entry = artifact
        .entrypoint
        .as_deref()
        .ok_or("distribution_layout_entrypoint_missing")?;
    if package.len() as u64 != artifact.package_size_bytes {
        return Err("distribution_package_size_mismatch");
    }
    budget(deadline, cancelled)?;
    let mut tree = unpack_package_archive_tree(
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
    verify_unpacked_bundle_tree(&tree, expected, deadline, cancelled)?;
    if let Some(bundle_hash) = artifact.bundle_tree_sha256.as_deref() {
        let root = artifact
            .bundle_archive_root
            .as_deref()
            .ok_or("distribution_bundle_mapping_missing")?;
        // 投影单独验证委托子树；随后转回原路径，所有字节只转移所有权。
        let projected = project_unpacked_bundle(tree, root, bundle_hash, deadline, cancelled)?;
        let (mut selected, mut remainder) = projected.into_parts();
        if !root.is_empty() {
            for file in &mut selected.files {
                budget(deadline, cancelled)?;
                file.relative_path = format!("{root}/{}", file.relative_path);
            }
            for directory in &mut selected.directories {
                budget(deadline, cancelled)?;
                *directory = format!("{root}/{directory}");
            }
            selected.directories.push(root.to_owned());
        }
        remainder.files.append(&mut selected.files);
        remainder.directories.append(&mut selected.directories);
        tree = remainder;
        verify_unpacked_bundle_tree(&tree, expected, deadline, cancelled)?;
    }
    let lock = parse_tool_lock_document(lock_bytes).map_err(|_| "distribution_lock_invalid")?;
    let tool = lock
        .find(tool_id, platform)
        .ok_or("distribution_layout_artifact_missing")?;
    budget(deadline, cancelled)?;
    Ok(DistributionLayout {
        tree,
        install_tree_sha256: expected.to_owned(),
        origin_ref: tool.origin_ref.clone(),
        bundle_root: tool.bundle.as_ref().map(|b| b.root.clone()),
        lock_sha256: manifest.lock_sha256.clone(),
        manifest_sha256: format!("{:x}", Sha256::digest(manifest_bytes)),
    })
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("distribution_layout_cancelled")
    } else if Instant::now() >= deadline {
        Err("distribution_layout_deadline_exceeded")
    } else {
        Ok(())
    }
}
