//! Doctor 临时私有运行区，不写项目。
use std::os::unix::fs::DirBuilderExt;
use std::{
    fs,
    path::{Path, PathBuf},
};
/// 独占 0700 目录，退出后清理。
pub(crate) struct DoctorScratch(PathBuf);
impl DoctorScratch {
    /// 使用内部运行 ID 创建新目录，拒绝复用。
    pub(crate) fn create(id: &str) -> Option<Self> {
        if id.is_empty() || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return None;
        }
        let path = std::env::temp_dir()
            .canonicalize()
            .ok()?
            .join(format!("codeguard-{id}"));
        fs::DirBuilder::new().mode(0o700).create(&path).ok()?;
        Some(Self(path))
    }
    /// 返回运行区路径。
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for DoctorScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
