//! 语法观察共享的普通 UTF-8 源码读取；拒绝链接祖先和超预算文件。
use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

/// 从指定路径读取至多 1 MiB 普通源码；返回原字节或具体路径/编码原因。
pub(crate) fn read_plain_source(path: &Path) -> Result<Vec<u8>, &'static str> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("source_path_parent_component_disallowed");
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "source_cwd_unavailable")?
            .join(path)
    };
    let mut prefix = PathBuf::new();
    for component in absolute.components() {
        prefix.push(component);
        let metadata = std::fs::symlink_metadata(&prefix).map_err(|_| "source_path_unavailable")?;
        if metadata.file_type().is_symlink() {
            return Err("source_path_symlink_disallowed");
        }
    }
    let metadata = std::fs::metadata(&absolute).map_err(|_| "source_unavailable")?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err("source_not_regular_or_too_large");
    }
    let mut source = Vec::new();
    File::open(&absolute)
        .map_err(|_| "source_unreadable")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut source)
        .map_err(|_| "source_unreadable")?;
    if source.len() > 1024 * 1024 || std::str::from_utf8(&source).is_err() {
        return Err("source_size_or_encoding_invalid");
    }
    Ok(source)
}
