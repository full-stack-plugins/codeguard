//! Clippy 的显式输入快照；只保护已观察范围，不证明完整 Cargo 生效模型。
use codeguard_runtime::SourceSnapshot;
use std::{
    collections::BTreeSet,
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// 绑定已观察 Rust 源码、根清单及可选配置的存在状态和字节。
/// 文件描述符快照拒绝链接父目录；根目录替换和可选配置新增也会使观察失效。
pub(crate) struct RustLintInputs {
    snapshot: SourceSnapshot,
    absent: Vec<PathBuf>,
    root_identity: (u64, u64),
}
impl RustLintInputs {
    /// 在原生启动前捕获指定源码和根配置；返回有界快照或不完整原因。
    pub(crate) fn capture(root: &Path, sources: &BTreeSet<String>) -> io::Result<Self> {
        if sources.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "缺少已观察 Rust 源码",
            ));
        }
        let metadata = fs::symlink_metadata(root)?;
        if !metadata.file_type().is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "项目根不是普通目录",
            ));
        }
        let root_identity = (metadata.dev(), metadata.ino());
        let mut paths: BTreeSet<PathBuf> = sources.iter().map(PathBuf::from).collect();
        paths.insert(PathBuf::from("Cargo.toml"));
        let mut absent = Vec::new();
        // 同时固定两种配置名称；不存在也必须固定，避免扫描中新增配置改变规则。
        for name in [
            "Cargo.lock",
            "clippy.toml",
            ".clippy.toml",
            "rust-toolchain",
            "rust-toolchain.toml",
            ".cargo/config",
            ".cargo/config.toml",
        ] {
            match fs::symlink_metadata(root.join(name)) {
                Ok(_) => {
                    paths.insert(PathBuf::from(name));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => absent.push(PathBuf::from(name)),
                Err(e) => return Err(e),
            }
        }
        if let Ok(metadata) = fs::symlink_metadata(root.join(".cargo")) {
            if !metadata.file_type().is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Cargo 配置目录不是普通目录",
                ));
            }
        }
        let snapshot =
            SourceSnapshot::capture(root, paths, 10000, 16 * 1024 * 1024, 64 * 1024 * 1024)?;
        Ok(Self {
            snapshot,
            absent,
            root_identity,
        })
    }
    /// 返回扫描前指定文件的原始字节，供诊断指纹和源码摘要绑定。
    pub(crate) fn source(&self, relative: &str) -> Option<&[u8]> {
        self.snapshot
            .files()
            .get(Path::new(relative))
            .map(Vec::as_slice)
    }
    /// 核对根身份、全部已捕获输入和可选文件不存在状态；错误视为未完成。
    pub(crate) fn unchanged(&self, root: &Path) -> bool {
        fs::symlink_metadata(root).is_ok_and(|metadata| {
            metadata.file_type().is_dir() && (metadata.dev(), metadata.ino()) == self.root_identity
        }) && self
            .snapshot
            .verify_source_unchanged()
            .is_ok_and(|same| same)
            && self.absent.iter().all(|relative| {
                fs::symlink_metadata(root.join(relative))
                    .is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
            })
    }
}
