//! 静态项目观察端口；领域层不自行读取文件系统或执行项目脚本。

use std::io;
use std::path::{Path, PathBuf};

/// 不跟随符号链接时观察到的路径类型。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservedPathKind {
    /// 普通文件。
    File,
    /// 目录。
    Directory,
    /// 符号链接，不能默认为普通源码。
    Symlink,
    /// 其它特殊文件。
    Other,
}

/// 可注入的只读文件系统观察能力；错误由调用者归入部分发现。
pub trait ObservationPort {
    /// 不跟随符号链接识别路径类型。
    fn classify(&self, path: &Path) -> io::Result<ObservedPathKind>;
    /// 仅列出直接子节点，不递归、不执行脚本。
    fn children(&self, directory: &Path) -> io::Result<Vec<PathBuf>>;
    /// 只读取大小受限的普通文件；不能跟随符号链接。
    fn read_bounded(&self, path: &Path, max_bytes: u64) -> io::Result<Vec<u8>>;
}
