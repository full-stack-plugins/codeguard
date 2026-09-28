//! 同机进程间的本地文件锁；状态解释与租约权限仍由应用服务负责。

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

/// 持有一个普通文件上的独占锁；进程结束时内核自动释放。
pub struct TaskFileLock(File);

impl TaskFileLock {
    /// 创建或打开指定锁文件并取得跨进程独占锁；拒绝符号链接和硬链接。
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

impl Drop for TaskFileLock {
    fn drop(&mut self) {
        // 关闭文件描述符也会释放锁；显式释放便于同进程顺序操作。
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}
