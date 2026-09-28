//! 显式包前缀映射；范围之外的文件仍返回给安装计划处理。
use crate::{
    ProjectedBundle, UnpackedArchive, package_archive::safe_path, verify_unpacked_bundle_tree,
};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 将完整归档中的明确目录映射为 bundle 根，并与预期摘要精确匹配。
/// 参数包含归档所有权、精确前缀/摘要及共享预算；返回只读投影和剩余成员。
/// 不复制文件内容、不写文件或批准来源；未匹配或非法输入没有局部成功。
pub fn project_unpacked_bundle(
    archive: UnpackedArchive,
    archive_root: &str,
    expected_tree_sha256: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<ProjectedBundle, &'static str> {
    budget(deadline, cancelled)?;
    archive.validate_shape()?;
    if !archive_root.is_empty()
        && (!safe_path(archive_root) || !archive.directories.iter().any(|dir| dir == archive_root))
    {
        return Err("bundle_archive_root_invalid");
    }
    budget(deadline, cancelled)?;
    let prefix = if archive_root.is_empty() {
        String::new()
    } else {
        format!("{archive_root}/")
    };
    let mut selected_files = Vec::new();
    let mut outside_files = Vec::new();
    for mut file in archive.files {
        budget(deadline, cancelled)?;
        if let Some(path) = file.relative_path.strip_prefix(&prefix) {
            file.relative_path = path.to_owned();
            selected_files.push(file);
        } else {
            outside_files.push(file);
        }
    }
    let mut selected_dirs = BTreeSet::new();
    let mut outside_dirs = BTreeSet::new();
    for directory in archive.directories {
        budget(deadline, cancelled)?;
        if !archive_root.is_empty() && directory == archive_root {
            continue;
        }
        if let Some(path) = directory.strip_prefix(&prefix) {
            selected_dirs.insert(path.to_owned());
        } else {
            outside_dirs.insert(directory);
        }
    }
    let tree = UnpackedArchive::from_parts(selected_files, selected_dirs)?;
    let remainder = UnpackedArchive::from_parts(outside_files, outside_dirs)?;
    verify_unpacked_bundle_tree(&tree, expected_tree_sha256, deadline, cancelled)?;
    budget(deadline, cancelled)?;
    Ok(ProjectedBundle::new(
        archive_root.to_owned(),
        expected_tree_sha256.to_owned(),
        tree,
        remainder,
    ))
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("bundle_projection_cancelled")
    } else if Instant::now() >= deadline {
        Err("bundle_projection_deadline_exceeded")
    } else {
        Ok(())
    }
}
