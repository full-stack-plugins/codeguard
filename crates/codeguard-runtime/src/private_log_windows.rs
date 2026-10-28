//! Windows evidence: protected DACL, pinned no-reparse ancestors, no-clobber
//! publication, bounded single-link report reads. No Unix-mode emulation.
use super::{NEXT_TEMP, ProcessOutcome, write_record};
use crate::{
    platform_fs,
    windows_path::{PinnedDirectory, file_info, wide},
};
use std::{
    fs::{self, File},
    io::{self, Read},
    os::windows::io::{AsRawHandle, FromRawHandle},
    path::{Path, PathBuf},
    ptr,
    sync::atomic::Ordering,
};
use windows_sys::Win32::{
    Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE},
    Storage::FileSystem::*,
};

pub struct FreshReportSlot {
    directory: PinnedDirectory,
    name: String,
    max_bytes: u64,
}
impl FreshReportSlot {
    pub fn read(&self) -> io::Result<Vec<u8>> {
        let path = wide(&self.directory.path().join(&self.name))?;
        // No write/delete sharing: the opened report cannot be replaced or
        // modified through another handle during this bounded read.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_handle(handle) };
        let info = file_info(file.as_raw_handle())?;
        let length = ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64;
        if info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
            || info.nNumberOfLinks != 1
            || length > self.max_bytes
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "report is not a bounded single-link regular file",
            ));
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "report exceeded byte limit",
            ));
        }
        Ok(bytes)
    }
}
pub fn prepare_fresh_report(
    root: &Path,
    name: &str,
    max_bytes: u64,
) -> io::Result<FreshReportSlot> {
    checked_name(name)?;
    if max_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "positive report limit required",
        ));
    }
    let directory = PinnedDirectory::open(root)?;
    platform_fs::verify_private_permissions(directory.path(), 0o700)?;
    match fs::symlink_metadata(directory.path().join(name)) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "old report cannot be reused",
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    Ok(FreshReportSlot {
        directory,
        name: name.into(),
        max_bytes,
    })
}
pub fn write_private_log(root: &Path, name: &str, outcome: &ProcessOutcome) -> io::Result<PathBuf> {
    checked_name(name)?;
    let directory = PinnedDirectory::open(root)?;
    platform_fs::verify_private_permissions(directory.path(), 0o700)?;
    let temporary = directory.path().join(format!(
        ".codeguard-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let target = directory.path().join(name);
    let mut file = platform_fs::secure_create_new(&temporary)?;
    let result = write_record(&mut file, outcome).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let from = wide(&temporary)?;
    let to = wide(&target)?;
    // WRITE_THROUGH waits for move completion; deliberately omit REPLACE_EXISTING.
    // Existing files, links and aliases are errors, never overwritten.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        let error = io::Error::last_os_error();
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(target)
}
fn checked_name(name: &str) -> io::Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        || reserved
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ordinary single Windows file name required",
        ));
    }
    Ok(())
}
