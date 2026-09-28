//! 与现有本机目录 bundle-tree-v1 完全相同的内存树记录顺序。
use crate::UnpackedArchive;
use ring::digest::{Context, SHA256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 对完整冻结树计算本机 bundle-tree-v1 摘要；参数含共享预算与取消。
/// 返回摘要或固定诊断，不落盘、不批准来源或证明依赖闭包完整。
pub fn hash_unpacked_bundle_tree(
    tree: &UnpackedArchive,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<String, &'static str> {
    budget(deadline, cancelled)?;
    tree.validate_shape()?;
    budget(deadline, cancelled)?;
    let directories = tree
        .directories
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let files = tree
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file.bytes.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for path in directories.iter().copied().chain(files.keys().copied()) {
        let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        children.entry(parent).or_default().push(path);
    }
    let mut hash = Context::new(&SHA256);
    hash.update(b"codeguard-bundle-tree-v1\0");
    let mut pending = vec![""];
    while let Some(directory) = pending.pop() {
        let Some(rows) = children.get_mut(directory) else {
            continue;
        };
        rows.sort_by(|left, right| Path::new(left).cmp(Path::new(right)));
        for path in rows.iter().copied() {
            budget(deadline, cancelled)?;
            let is_directory = directories.contains(path);
            hash.update(if is_directory { b"D" } else { b"F" });
            hash.update(&(path.len() as u64).to_be_bytes());
            hash.update(path.as_bytes());
            if is_directory {
                pending.push(path);
                continue;
            }
            let bytes = files.get(path).ok_or("bundle_parent_missing")?;
            if bytes.len() > 128 * 1024 * 1024 {
                return Err("bundle_file_limit");
            }
            hash.update(&(bytes.len() as u64).to_be_bytes());
            let mut content = Context::new(&SHA256);
            for chunk in bytes.chunks(128 * 1024) {
                budget(deadline, cancelled)?;
                content.update(chunk);
            }
            hash.update(content.finish().as_ref());
        }
    }
    budget(deadline, cancelled)?;
    Ok(hash
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
/// 将重新计算的树摘要与预期锁摘要比较；成功仅是完整树内容绑定。
/// 拒绝非法/零摘要，支持取消和期限，不构造发行授权或安装收据。
pub fn verify_unpacked_bundle_tree(
    tree: &UnpackedArchive,
    expected: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<(), &'static str> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        || expected.bytes().all(|b| b == b'0')
    {
        return Err("bundle_expected_digest_invalid");
    }
    if hash_unpacked_bundle_tree(tree, deadline, cancelled)? != expected {
        return Err("bundle_digest_mismatch");
    }
    Ok(())
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("bundle_cancelled")
    } else if Instant::now() >= deadline {
        Err("bundle_deadline_exceeded")
    } else {
        Ok(())
    }
}
