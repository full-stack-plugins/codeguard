//! Doctor 临时私有运行区，不写项目。
use std::os::unix::fs::DirBuilderExt;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// 独占 0700 目录，退出后清理。
pub(crate) struct DoctorScratch(PathBuf);
impl DoctorScratch {
    /// 使用内部运行 ID 创建新目录，拒绝复用。
    pub(crate) fn create(id: &str) -> Option<Self> {
        if id.is_empty() || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return None;
        }
        let path = std::env::temp_dir().canonicalize().ok()?.join(format!(
            "codeguard-{id}-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
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

#[cfg(test)]
mod tests {
    use super::DoctorScratch;
    use std::collections::BTreeSet;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_same_label_creates_distinct_private_directories() {
        let barrier = Arc::new(Barrier::new(8));
        let handles = (0..8)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    DoctorScratch::create("parallel-native-check")
                        .expect("每轮并发检查都有独占私有目录")
                })
            })
            .collect::<Vec<_>>();
        let scratches = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let paths = scratches
            .iter()
            .map(|scratch| scratch.path().to_path_buf())
            .collect::<BTreeSet<_>>();
        assert_eq!(paths.len(), 8);
        assert!(paths.iter().all(|path| path.is_dir()));
        assert!(paths.iter().all(|path| {
            std::fs::metadata(path)
                .is_ok_and(|metadata| metadata.permissions().mode() & 0o777 == 0o700)
        }));
        drop(scratches);
        assert!(paths.iter().all(|path| !path.exists()));
    }
}
