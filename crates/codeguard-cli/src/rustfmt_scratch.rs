use codeguard_runtime::SourceSnapshot;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// 显式 Rustfmt 解析的私有固定配置；来源：OpenSpec 隔离原生语法差分契约。
pub(crate) struct RustfmtScratch {
    pub(crate) root: PathBuf,
    snapshot: SourceSnapshot,
}
impl RustfmtScratch {
    /// 独占创建配置目录并固定 edition；返回配置与原字节快照，不读取项目配置。
    #[cfg(test)]
    pub(crate) fn create() -> Option<Self> {
        Self::create_for_edition("2024")
    }
    /// 创建指定edition的私有配置；只接受已支持的Cargo edition，不读取项目配置。
    pub(crate) fn create_for_edition(edition: &str) -> Option<Self> {
        if !matches!(edition, "2015" | "2018" | "2021" | "2024") {
            return None;
        }
        let parent = std::env::temp_dir().canonicalize().ok()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let root = parent.join(format!(
            "cg-rustfmt-{}-{now}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).ok()?;
        let setup = (|| {
            let mut config = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(root.join("fixed.toml"))
                .ok()?;
            config
                .write_all(format!("edition = \"{edition}\"\n").as_bytes())
                .ok()?;
            let snapshot =
                SourceSnapshot::capture(&root, [PathBuf::from("fixed.toml")], 1, 1024, 1024)
                    .ok()?;
            Some(snapshot)
        })();
        match setup {
            Some(snapshot) => Some(Self { root, snapshot }),
            None => {
                let _ = fs::remove_dir_all(&root);
                None
            }
        }
    }
    /// 核对私有配置的原字节及路径身份；读取失败视为配置变化。
    pub(crate) fn current(&self) -> bool {
        self.snapshot
            .verify_source_unchanged()
            .is_ok_and(|same| same)
    }
    /// 返回固定文件路径，仅供验收或身份说明，不传入项目配置。
    #[cfg(test)]
    pub(crate) fn config_path(&self) -> impl AsRef<std::path::Path> {
        self.root.join("fixed.toml")
    }
}
impl Drop for RustfmtScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
