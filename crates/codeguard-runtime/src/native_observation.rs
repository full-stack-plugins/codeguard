//! 本地文件系统的只读观察端口实现。

use codeguard_core::{ObservationPort, ObservedPathKind};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 不执行进程、不跟随末尾符号链接的静态观察器。
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeObservation;

impl ObservationPort for NativeObservation {
    fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
        let kind = fs::symlink_metadata(path)?.file_type();
        Ok(if kind.is_file() {
            ObservedPathKind::File
        } else if kind.is_dir() {
            ObservedPathKind::Directory
        } else if kind.is_symlink() {
            ObservedPathKind::Symlink
        } else {
            ObservedPathKind::Other
        })
    }

    fn children(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect()
    }

    fn read_bounded(&self, path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
        crate::read_bounded_regular_file(path, max_bytes)
    }
}
