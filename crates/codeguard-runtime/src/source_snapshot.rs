//! 原生检查器的有界源码输入快照；范围完整性由调用方独立证明。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(not(unix))]
use crate::read_bounded_regular_file;

/// 一组已读取的普通文件字节，用于私有检查工作区和运行后复核。
#[derive(Debug)]
pub struct SourceSnapshot {
    #[cfg(not(unix))]
    root: PathBuf,
    #[cfg(unix)]
    root_fd: OwnedFd,
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl SourceSnapshot {
    /// 按显式相对路径读取文件；参数是根目录、文件路径、文件数及字节预算。
    /// 返回仅代表这些文件的字节快照，不代表已发现完整项目源集。
    pub fn capture(
        root: &Path,
        paths: impl IntoIterator<Item = PathBuf>,
        max_files: usize,
        max_file_bytes: u64,
        max_total_bytes: u64,
    ) -> io::Result<Self> {
        if max_files == 0 {
            return Err(invalid("快照根目录或文件预算无效"));
        }
        #[cfg(unix)]
        let root_fd = open_directory(root)?;
        #[cfg(not(unix))]
        let root = {
            let root_type = fs::symlink_metadata(root)?.file_type();
            if !root_type.is_dir() || root_type.is_symlink() {
                return Err(invalid("快照根目录不是普通目录"));
            }
            root.canonicalize()?
        };
        let mut files = BTreeMap::new();
        let mut total = 0_u64;
        for relative in paths {
            validate_relative(&relative)?;
            if files.len() >= max_files || files.contains_key(&relative) {
                return Err(invalid("文件数量超限或路径重复"));
            }
            #[cfg(unix)]
            let content = read_relative(&root_fd, &relative, max_file_bytes)?;
            #[cfg(not(unix))]
            let content =
                read_bounded_regular_file(&checked_source_path(&root, &relative)?, max_file_bytes)?;
            total = total
                .checked_add(content.len() as u64)
                .ok_or_else(|| invalid("快照字节数溢出"))?;
            if total > max_total_bytes {
                return Err(invalid("快照总字节数超限"));
            }
            files.insert(relative, content);
        }
        if files.is_empty() {
            return Err(invalid("空输入不能证明项目源集"));
        }
        Ok(Self {
            #[cfg(not(unix))]
            root,
            #[cfg(unix)]
            root_fd,
            files,
        })
    }

    /// 将快照写入必须尚不存在的私有目录；返回写入错误而不覆盖旧产物。
    pub fn materialize_new(&self, destination: &Path) -> io::Result<()> {
        fs::create_dir(destination)?;
        for (relative, bytes) in &self.files {
            let target = destination.join(relative);
            fs::create_dir_all(target.parent().ok_or_else(|| invalid("目标路径无父目录"))?)?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)?;
            use std::io::Write;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Ok(())
    }

    /// 重新读取源文件与私有副本，确保原生执行没有改变本轮输入。
    /// 参数为先前创建的副本目录；返回两侧均与捕获字节一致的结果。
    pub fn verify_unchanged(&self, destination: &Path) -> io::Result<bool> {
        #[cfg(unix)]
        let destination_fd = open_directory(destination)?;
        #[cfg(not(unix))]
        if !fs::symlink_metadata(destination)?.file_type().is_dir() {
            return Ok(false);
        }
        for (relative, expected) in &self.files {
            let limit = expected.len() as u64;
            #[cfg(unix)]
            let unchanged = read_relative(&self.root_fd, relative, limit)? == *expected
                && read_relative(&destination_fd, relative, limit)? == *expected;
            #[cfg(not(unix))]
            let unchanged =
                read_bounded_regular_file(&checked_source_path(&self.root, relative)?, limit)?
                    == *expected
                    && read_bounded_regular_file(
                        &checked_source_path(destination, relative)?,
                        limit,
                    )? == *expected;
            if !unchanged {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 仅复核原工作区的显式源码字节；用于本地全项目扫描的运行后漂移观察。
    /// 返回 `false` 表示普通文件内容变化；路径不可安全读取仍返回错误。
    pub fn verify_source_unchanged(&self) -> io::Result<bool> {
        for (relative, expected) in &self.files {
            let limit = expected.len() as u64;
            #[cfg(unix)]
            let actual = read_relative(&self.root_fd, relative, limit)?;
            #[cfg(not(unix))]
            let actual =
                read_bounded_regular_file(&checked_source_path(&self.root, relative)?, limit)?;
            if actual != *expected {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 取得快照中的显式文件与原始字节，供原生报告归属使用。
    pub fn files(&self) -> &BTreeMap<PathBuf, Vec<u8>> {
        &self.files
    }
}

fn validate_relative(relative: &Path) -> io::Result<()> {
    let normalized: PathBuf = relative.components().collect();
    if relative.as_os_str().is_empty()
        || normalized.as_os_str() != relative.as_os_str()
        || relative.components().count() > 64
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(invalid("输入路径必须是普通仓库相对路径"));
    }
    Ok(())
}

#[cfg(not(unix))]
fn checked_source_path(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let mut path = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        path.push(component.as_os_str());
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink()
            || (components.peek().is_some() && !kind.is_dir())
            || (components.peek().is_none() && !kind.is_file())
        {
            return Err(invalid("快照路径包含链接或非普通文件"));
        }
    }
    Ok(path)
}

#[cfg(unix)]
fn open_directory(path: &Path) -> io::Result<OwnedFd> {
    let name =
        CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("目录路径含 NUL 字节"))?;
    // SAFETY: C 字符串有效；返回的文件描述符由 OwnedFd 接管。
    let raw = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: open 成功后 raw 是当前函数独占的新文件描述符。
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

#[cfg(unix)]
fn read_relative(root: &OwnedFd, relative: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    let mut directories = Vec::new();
    let mut parent = root.as_raw_fd();
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        let name = CString::new(part.as_os_str().as_bytes())
            .map_err(|_| invalid("相对路径含 NUL 字节"))?;
        let last = parts.peek().is_none();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if last {
                libc::O_NONBLOCK
            } else {
                libc::O_DIRECTORY
            };
        // SAFETY: parent 在本轮循环中仍由 root 或 directory 持有；name 是有效 C 字符串。
        let raw = unsafe { libc::openat(parent, name.as_ptr(), flags) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat 成功后 raw 是新文件描述符，交由 OwnedFd 关闭。
        let opened = unsafe { OwnedFd::from_raw_fd(raw) };
        if last {
            let mut file = fs::File::from(opened);
            let metadata = file.metadata()?;
            if !metadata.file_type().is_file() || metadata.len() > max_bytes {
                return Err(invalid("快照输入不是有界普通文件"));
            }
            let mut bytes = Vec::new();
            file.by_ref()
                .take(max_bytes.saturating_add(1))
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > max_bytes {
                return Err(invalid("快照输入超过大小上限"));
            }
            return Ok(bytes);
        }
        directories.push(opened);
        parent = directories.last().expect("已保存目录描述符").as_raw_fd();
    }
    Err(invalid("空快照路径"))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
