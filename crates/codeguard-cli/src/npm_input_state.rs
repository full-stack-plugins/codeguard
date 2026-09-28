//! 有界观察npm输入状态；不读取链接目标，不将不可读当作不存在。
use codeguard_runtime::read_bounded_regular_file;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 参数为输入物理路径与字节上限；返回状态及仅可读普通文件具有的摘要。
pub(crate) fn observe(path: &Path, limit: u64) -> (&'static str, Option<String>) {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ("missing", None),
        Err(_) => return ("unavailable", None),
        Ok(m) if m.file_type().is_symlink() => return ("path_alias", None),
        Ok(m) if !m.is_file() => return ("not_regular", None),
        Ok(_) => {}
    }
    if path.canonicalize().ok().as_deref() != Some(path) {
        return ("path_alias", None);
    }
    match read_bounded_regular_file(path, limit) {
        Ok(bytes) => ("present", Some(format!("{:x}", Sha256::digest(bytes)))),
        Err(_) => ("unavailable", None),
    }
}
