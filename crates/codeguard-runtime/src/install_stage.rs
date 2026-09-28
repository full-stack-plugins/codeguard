//! 持有同一缓存目录描述符的独占暂存文件。
use std::{
    ffi::CString,
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// 正常返回与失败路径都清理暂存名字，不跟随可替换的目录路径。
pub(crate) struct InstallStage {
    directory: File,
    pub(crate) name: CString,
    pub(crate) file: File,
}
impl InstallStage {
    /// 在已核验目录内创建有界重试的独占暂存；不覆盖崩溃遗留文件。
    pub(crate) fn create(directory: &File) -> Result<Self, &'static str> {
        for _ in 0..64 {
            let name = CString::new(format!(
                ".install-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ))
            .map_err(|_| "install_stage_unavailable")?;
            // 目录 FD 固定归属；O_EXCL/O_NOFOLLOW 防止暂存名字被链接接管。
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_RDWR,
                    0o700,
                )
            };
            if fd >= 0 {
                // fd 由 openat 新建且只在本对象中关闭。
                let file = unsafe { File::from_raw_fd(fd) };
                let directory = match directory.try_clone() {
                    Ok(value) => value,
                    Err(_) => {
                        unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
                        return Err("install_stage_unavailable");
                    }
                };
                return Ok(Self {
                    directory,
                    name,
                    file,
                });
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err("install_stage_unavailable");
            }
        }
        Err("install_stage_unavailable")
    }
}
impl Drop for InstallStage {
    fn drop(&mut self) {
        // 只移除本次独占创建的名字；最终内容寻址条目保持。
        unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
    }
}
