//! Pin every directory component while a path-based Windows API operates.
//! Handles omit write/delete sharing, so verified ancestors cannot be modified, renamed or
//! replaced until the guard drops. Reparse points and device/UNC roots are rejected.
use std::os::windows::io::AsRawHandle;
use std::{
    fs::File,
    io,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
    path::{Component, Path, PathBuf, Prefix},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::*,
};

pub(crate) struct PinnedDirectory {
    path: PathBuf,
    _handles: Vec<File>,
}
impl PinnedDirectory {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Err(invalid("absolute disk path required"));
        };
        if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            || !matches!(components.next(), Some(Component::RootDir))
        {
            return Err(invalid("local absolute disk path required"));
        }
        let mut current = PathBuf::from(prefix.as_os_str());
        current.push(std::path::MAIN_SEPARATOR_STR);
        let mut handles = vec![open_directory(&current)?];
        for part in components {
            let Component::Normal(name) = part else {
                return Err(invalid("special path component rejected"));
            };
            if name.to_string_lossy().contains(':') {
                return Err(invalid("alternate data stream rejected"));
            }
            current.push(name);
            handles.push(open_directory(&current)?);
        }
        Ok(Self {
            path: current,
            _handles: handles,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}
fn open_directory(path: &Path) -> io::Result<File> {
    let path = wide(path)?;
    // SAFETY: NUL-terminated UTF-16 path, no inherited handle. Omitting write/delete
    // sharing pins both directory identity and reparse metadata. OPEN_REPARSE_POINT observes the link itself.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_handle(handle) };
    let info = file_info(file.as_raw_handle())?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(invalid("path component is not a real directory"));
    }
    Ok(file)
}
pub(crate) fn file_info(handle: HANDLE) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}
pub(crate) fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(invalid("NUL in path"));
    }
    value.push(0);
    Ok(value)
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
