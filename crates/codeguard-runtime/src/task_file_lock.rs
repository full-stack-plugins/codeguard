//! 同机进程间的本地文件锁；状态解释与租约权限仍由应用服务负责。

use std::fs::File;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::io;
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

/// 持有一个普通文件上的独占锁；进程结束时内核自动释放。
pub struct TaskFileLock(File, #[cfg(windows)] crate::windows_path::PinnedDirectory);

impl TaskFileLock {
    /// 创建或打开指定锁文件并取得跨进程独占锁；拒绝符号链接和硬链接。
    #[cfg(unix)]
    pub fn acquire(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file() || metadata.nlink() != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "锁文件身份无效"));
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(file))
    }
}

#[cfg(unix)]
impl Drop for TaskFileLock {
    fn drop(&mut self) {
        // 关闭文件描述符也会释放锁；显式释放便于同进程顺序操作。
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(windows)]
impl TaskFileLock {
    /// An exclusive share-mode handle is a nonblocking kernel lock. Every user
    /// of this protocol opens the same file with sharing denied; process exit
    /// closes the handle and releases exclusion, including crash recovery.
    pub fn acquire(path: &Path) -> io::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::*;
        use windows_sys::{
            Wdk::Storage::FileSystem::{FILE_NON_DIRECTORY_FILE, FILE_OPEN},
            Win32::Foundation::{GENERIC_READ, GENERIC_WRITE},
        };
        let parent = crate::windows_path::PinnedDirectory::open(
            path.parent()
                .ok_or_else(|| io::Error::other("lock parent required"))?,
        )?;
        match crate::platform_fs::secure_create_new(path) {
            Ok(file) => drop(file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let file = parent.open_child(
            path.file_name()
                .ok_or_else(|| io::Error::other("lock leaf required"))?,
            GENERIC_READ | GENERIC_WRITE,
            0,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE,
            std::ptr::null_mut(),
        )?;
        let mut info = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
            || info.nNumberOfLinks != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "lock file identity invalid",
            ));
        }
        Ok(Self(file, parent))
    }
}
