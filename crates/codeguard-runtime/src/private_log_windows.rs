//! Windows evidence: protected DACL, pinned no-reparse ancestors, no-clobber
//! publication, bounded single-link report reads. No Unix-mode emulation.
use super::{NEXT_TEMP, ProcessOutcome, write_record};
use crate::{
    platform_fs,
    windows_path::{PinnedDirectory, file_info},
};
use std::{
    io::{self, Read},
    os::windows::io::AsRawHandle,
    path::{Path, PathBuf},
    ptr,
    sync::atomic::Ordering,
};
use windows_sys::Wdk::Storage::FileSystem::{FILE_NON_DIRECTORY_FILE, FILE_OPEN};
use windows_sys::Win32::{Foundation::GENERIC_READ, Storage::FileSystem::*};

pub struct FreshReportSlot {
    directory: PinnedDirectory,
    name: String,
    max_bytes: u64,
}
impl FreshReportSlot {
    pub fn read(&self) -> io::Result<Vec<u8>> {
        // Handle-relative lookup cannot follow a mutated ancestor. No write or
        // delete sharing keeps the opened report stable during the bounded read.
        let file = self.directory.open_child(
            std::ffi::OsStr::new(&self.name),
            GENERIC_READ,
            FILE_SHARE_READ,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE,
            ptr::null_mut(),
        )?;
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
    match directory.open_child(
        std::ffi::OsStr::new(name),
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_NON_DIRECTORY_FILE,
        ptr::null_mut(),
    ) {
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
    let directory = PinnedDirectory::open_with_access(root, FILE_ADD_FILE)?;
    platform_fs::verify_private_permissions(directory.path(), 0o700)?;
    let temporary_name = format!(
        ".codeguard-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    );
    let temporary = directory.path().join(&temporary_name);
    let target = directory.path().join(name);
    let mut file = platform_fs::secure_create_new(&temporary)?;
    let result = write_record(&mut file, outcome).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = result {
        discard_staging(&directory, &temporary_name);
        return Err(error);
    }
    let file = directory.open_child(
        std::ffi::OsStr::new(&temporary_name),
        DELETE | windows_sys::Win32::Foundation::GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_NON_DIRECTORY_FILE,
        ptr::null_mut(),
    )?;
    // RootDirectory+leaf rename never re-resolves mutable ancestors. false is a
    // strict no-clobber contract, including existing links and old evidence.
    if let Err(error) =
        platform_fs::rename_bound(&file, &directory, std::ffi::OsStr::new(name), false)
    {
        drop(file);
        discard_staging(&directory, &temporary_name);
        return Err(error);
    }
    file.sync_all()?;
    Ok(target)
}
fn discard_staging(directory: &PinnedDirectory, name: &str) {
    // Best-effort only against the already anchored directory. A reparse race
    // leaves a private orphan rather than deleting through an untrusted path.
    if let Ok(file) = directory.open_child(
        std::ffi::OsStr::new(name),
        DELETE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        FILE_NON_DIRECTORY_FILE,
        ptr::null_mut(),
    ) {
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                FileDispositionInfo,
                (&info as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            );
        }
    }
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
