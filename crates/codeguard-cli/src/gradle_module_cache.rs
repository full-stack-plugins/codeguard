use codeguard_runtime::SourceSnapshot;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 读取显式modules-2缓存快照；参数为已有缓存、截止时间与取消信号，返回有界只读字节。
/// 不加载用户配置，不跟随符号链接，不复制锁文件，也不修改用户缓存。
pub(crate) fn capture(
    root: &Path,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<SourceSnapshot, &'static str> {
    let metadata = fs::symlink_metadata(root).map_err(|_| "gradle_dependency_cache_unavailable")?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || root.file_name().and_then(|part| part.to_str()) != Some("modules-2")
    {
        return Err("gradle_dependency_cache_scope_invalid");
    }
    let mut stack = vec![PathBuf::new()];
    let mut files = BTreeSet::new();
    let mut visits = 0;
    let mut total = 0_u64;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(root.join(&directory))
            .map_err(|_| "gradle_dependency_cache_unavailable")?
        {
            if Instant::now() >= deadline
                || cancelled.load(Ordering::Relaxed)
                || codeguard_runtime::sigint_cancellation_requested()
            {
                return Err("gradle_dependency_cache_capture_interrupted");
            }
            visits += 1;
            if visits > 20_000 {
                return Err("gradle_dependency_cache_budget_exceeded");
            }
            let entry = entry.map_err(|_| "gradle_dependency_cache_unavailable")?;
            let relative = directory.join(entry.file_name());
            if directory.as_os_str().is_empty() {
                let name = entry.file_name();
                let name = name
                    .to_str()
                    .ok_or("gradle_dependency_cache_scope_invalid")?;
                if matches!(name, "modules-2.lock" | "gc.properties") {
                    continue;
                }
                if name != "files-2.1"
                    && name != "resources-2.1"
                    && !name.strip_prefix("metadata-2.").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                    })
                {
                    return Err("gradle_dependency_cache_scope_invalid");
                }
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| "gradle_dependency_cache_unavailable")?;
            if metadata.file_type().is_symlink() {
                return Err("gradle_dependency_cache_scope_invalid");
            }
            if metadata.is_dir() {
                stack.push(relative);
            } else if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or("gradle_dependency_cache_budget_exceeded")?;
                if metadata.len() > 128 * 1024 * 1024
                    || total > 512 * 1024 * 1024
                    || files.len() >= 4096
                {
                    return Err("gradle_dependency_cache_budget_exceeded");
                }
                files.insert(relative);
            } else {
                return Err("gradle_dependency_cache_scope_invalid");
            }
        }
    }
    SourceSnapshot::capture(root, files, 4096, 128 * 1024 * 1024, 512 * 1024 * 1024)
        .map_err(|_| "gradle_dependency_cache_snapshot_unavailable")
}
