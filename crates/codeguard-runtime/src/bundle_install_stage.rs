//! 工具目录的独占暂存；清理只访问本次已创建的规范成员。
use crate::bundle_cache_fs::{name, open_dir, parent};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// 固定暂存和父目录描述符；发布后解除清理，失败不跟随替换路径。
pub(crate) struct BundleInstallStage {
    cache: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) files: Vec<String>,
    pub(crate) directories: Vec<String>,
    pub(crate) published: bool,
}
impl BundleInstallStage {
    /// 在已固定缓存内独占创建私有目录；不接管遗留暂存。
    pub(crate) fn create(cache: &File) -> Result<Self, &'static str> {
        let owned = cache
            .try_clone()
            .map_err(|_| "bundle_install_stage_unavailable")?;
        for _ in 0..64 {
            let value = format!(
                ".bundle-install-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let name = name(&value)?;
            if unsafe { libc::mkdirat(cache.as_raw_fd(), name.as_ptr(), 0o700) } == 0 {
                let directory = match open_dir(cache, &value) {
                    Ok(f) => f,
                    Err(e) => {
                        unsafe {
                            libc::unlinkat(cache.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR)
                        };
                        return Err(e);
                    }
                };
                return Ok(Self {
                    cache: owned,
                    directory,
                    name,
                    files: Vec::new(),
                    directories: Vec::new(),
                    published: false,
                });
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err("bundle_install_stage_unavailable");
            }
        }
        Err("bundle_install_stage_unavailable")
    }
}
impl Drop for BundleInstallStage {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        for path in self.files.iter().rev() {
            if let Ok((fd, leaf)) = parent(&self.directory, path) {
                unsafe { libc::unlinkat(fd.as_raw_fd(), leaf.as_ptr(), 0) };
            }
        }
        for path in self.directories.iter().rev() {
            if let Ok((fd, leaf)) = parent(&self.directory, path) {
                unsafe { libc::unlinkat(fd.as_raw_fd(), leaf.as_ptr(), libc::AT_REMOVEDIR) };
            }
        }
        // 缓存中的暂存名字若被替换，不移除替代目录。
        let Ok(text) = self.name.to_str() else {
            return;
        };
        if let Ok(current) = open_dir(&self.cache, text) {
            if let (Ok(a), Ok(b)) = (current.metadata(), self.directory.metadata()) {
                if a.dev() == b.dev() && a.ino() == b.ino() {
                    unsafe {
                        libc::unlinkat(
                            self.cache.as_raw_fd(),
                            self.name.as_ptr(),
                            libc::AT_REMOVEDIR,
                        )
                    };
                }
            }
        }
    }
}
