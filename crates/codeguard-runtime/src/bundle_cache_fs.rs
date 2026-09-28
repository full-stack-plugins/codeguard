//! 基于固定目录描述符的工具树 I/O；路径成员必须已通过完整树校验。
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
pub(crate) fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("bundle_install_cancelled")
    } else if Instant::now() >= deadline {
        Err("bundle_install_deadline_exceeded")
    } else {
        Ok(())
    }
}
pub(crate) fn name(value: &str) -> Result<CString, &'static str> {
    CString::new(value).map_err(|_| "bundle_install_path_invalid")
}
pub(crate) fn open_dir(parent: &File, value: &str) -> Result<File, &'static str> {
    let name = name(value)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err("bundle_install_directory_invalid");
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub(crate) fn parent(root: &File, path: &str) -> Result<(File, CString), &'static str> {
    let mut parts: Vec<_> = path.split('/').collect();
    let leaf = name(parts.pop().ok_or("bundle_install_path_invalid")?)?;
    let mut directory = root
        .try_clone()
        .map_err(|_| "bundle_install_directory_invalid")?;
    for part in parts {
        directory = open_dir(&directory, part)?;
    }
    Ok((directory, leaf))
}
pub(crate) fn private(file: &File, directory: bool) -> Result<(), &'static str> {
    let m = file
        .metadata()
        .map_err(|_| "bundle_install_entry_invalid")?;
    if m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o7777 != 0o700
        || if directory {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1
        }
    {
        return Err("bundle_install_entry_invalid");
    }
    Ok(())
}
pub(crate) fn children(
    directory: &File,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, &'static str> {
    // 重新打开点目录取得独立游标，避免 dup 的共享目录偏移影响下一次核验。
    let fd = open_dir(directory, ".")?;
    let raw = std::os::fd::IntoRawFd::into_raw_fd(fd);
    let stream = unsafe { libc::fdopendir(raw) };
    if stream.is_null() {
        unsafe { libc::close(raw) };
        return Err("bundle_install_directory_invalid");
    }
    let result = (|| {
        let mut names = Vec::new();
        loop {
            budget(deadline, cancelled)?;
            // readdir 的空指针既可能是结束也可能是错误，必须检查线程 errno。
            #[cfg(target_os = "macos")]
            let error = unsafe { libc::__error() };
            #[cfg(target_os = "linux")]
            let error = unsafe { libc::__errno_location() };
            unsafe { *error = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                if unsafe { *error } != 0 {
                    return Err("bundle_install_directory_read_failed");
                }
                break;
            }
            let text = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_str()
                .map_err(|_| "bundle_install_entry_invalid")?;
            if matches!(text, "." | "..") {
                continue;
            }
            if names.len() >= 100_000 {
                return Err("bundle_install_entry_limit");
            }
            names.push(text.to_owned());
        }
        names.sort();
        Ok(names)
    })();
    unsafe { libc::closedir(stream) };
    result
}
