//! Handle-relative Windows directory traversal. Each component is opened against
//! the preceding directory handle, never re-resolved through a mutable full path.
//! Rename protection uses real directory access and no delete sharing. Write
//! sharing remains enabled so unrelated child creation/rename can proceed.
use std::{
    ffi::OsStr,
    fs::File,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr,
};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
            NtCreateFile,
        },
    },
    Win32::{
        Foundation::{
            HANDLE, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
            RtlNtStatusToDosError, UNICODE_STRING,
        },
        Storage::FileSystem::*,
        System::IO::IO_STATUS_BLOCK,
    },
};

pub(crate) struct PinnedDirectory {
    path: PathBuf,
    handles: Vec<File>,
}
impl PinnedDirectory {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Self::open_with_access(path, 0)
    }
    pub(crate) fn open_with_access(path: &Path, final_access: u32) -> io::Result<Self> {
        let mut parts = path.components();
        let Some(Component::Prefix(prefix)) = parts.next() else {
            return Err(invalid("absolute disk path required"));
        };
        if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            || !matches!(parts.next(), Some(Component::RootDir))
        {
            return Err(invalid("local absolute disk path required"));
        }
        let mut current = PathBuf::from(prefix.as_os_str());
        current.push(std::path::MAIN_SEPARATOR_STR);
        let rest: Vec<_> = parts.collect();
        let root_access = if rest.is_empty() { final_access } else { 0 };
        let mut handles = vec![open_volume_root(&current, root_access)?];
        for (index, part) in rest.iter().enumerate() {
            let Component::Normal(name) = part else {
                return Err(invalid("special path component rejected"));
            };
            let extra = if index + 1 == rest.len() {
                final_access
            } else {
                0
            };
            let file = open_relative(
                handles.last().unwrap().as_raw_handle(),
                name,
                FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | extra,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_OPEN,
                FILE_DIRECTORY_FILE,
                ptr::null_mut(),
            )?;
            current.push(name);
            handles.push(file);
        }
        Ok(Self {
            path: current,
            handles,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn handle(&self) -> HANDLE {
        self.handles
            .last()
            .expect("root handle exists")
            .as_raw_handle()
    }
    pub(crate) fn open_child(
        &self,
        name: &OsStr,
        access: u32,
        share: u32,
        disposition: u32,
        options: u32,
        descriptor: *mut core::ffi::c_void,
    ) -> io::Result<File> {
        // Metadata changes to any retained ancestor are explicit failures; the
        // lookup itself is rooted in the retained final handle regardless.
        for directory in &self.handles {
            real_directory(directory.as_raw_handle())?;
        }
        open_relative(
            self.handle(),
            name,
            access,
            share,
            disposition,
            options,
            descriptor,
        )
    }
}
fn real_directory(handle: HANDLE) -> io::Result<()> {
    let info = file_info(handle)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(invalid("path component is not a real directory"));
    }
    Ok(())
}
fn open_volume_root(path: &Path, extra: u32) -> io::Result<File> {
    let name = wide(path)?;
    // Only a drive root is resolved with the DOS API; all user-controlled
    // components below it use NtCreateFile RootDirectory handles.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | extra,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
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
    real_directory(file.as_raw_handle())?;
    Ok(file)
}
fn open_relative(
    parent: HANDLE,
    name: &OsStr,
    access: u32,
    share: u32,
    disposition: u32,
    options: u32,
    descriptor: *mut core::ffi::c_void,
) -> io::Result<File> {
    let mut name: Vec<u16> = name.encode_wide().collect();
    if name.is_empty()
        || name.len() > 32766
        || name.iter().any(|c| matches!(*c, 0 | 47 | 92 | 58))
        || name == [46]
        || name == [46, 46]
    {
        return Err(invalid("single relative file name required"));
    }
    let length = (name.len() * 2) as u16;
    let mut unicode = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: name.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent,
        ObjectName: &mut unicode,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: descriptor.cast(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut result = ptr::null_mut();
    let mut status_block = IO_STATUS_BLOCK::default();
    // SAFETY: bounded UTF-16 leaf, live parent handle, initialized NT structs.
    // The synchronous non-alert handle is required by std File's blocking I/O.
    // Request FILE_READ_ATTRIBUTES for the mandatory post-open identity/reparse
    // query, even when the caller only requests WRITE_DAC/READ_CONTROL or write
    // data. Native NtCreateFile does not add that right on the caller's behalf.
    let status = unsafe {
        NtCreateFile(
            &mut result,
            access | SYNCHRONIZE | FILE_READ_ATTRIBUTES,
            &attributes,
            &mut status_block,
            ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            share,
            disposition,
            options | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        ));
    }
    let file = unsafe { File::from_raw_handle(result) };
    if file_info(file.as_raw_handle())?.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(invalid("reparse point rejected"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use windows_sys::{
        Wdk::Storage::FileSystem::{FILE_CREATE, FILE_NON_DIRECTORY_FILE},
        Win32::Foundation::GENERIC_WRITE,
    };

    #[test]
    fn acl_only_and_write_only_child_requests_still_support_required_identity_query() {
        let root = std::env::temp_dir().join(format!(
            "cg-nt-query-rights-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        crate::platform_fs::create_private_dir(&root).unwrap();
        let directory = PinnedDirectory::open(&root).unwrap();
        // This access request is used by private ACL verification. The backend
        // must also obtain its own metadata-query right; the caller needn't know
        // implementation-specific post-open identity validation requirements.
        let parent = PinnedDirectory::open(root.parent().unwrap()).unwrap();
        let acl_handle = parent
            .open_child(
                root.file_name().unwrap(),
                READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                FILE_OPEN,
                FILE_DIRECTORY_FILE,
                ptr::null_mut(),
            )
            .unwrap();
        assert_ne!(
            file_info(acl_handle.as_raw_handle())
                .unwrap()
                .dwFileAttributes
                & FILE_ATTRIBUTE_DIRECTORY,
            0
        );
        let mut file = directory
            .open_child(
                OsStr::new("new.bin"),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_DELETE,
                FILE_CREATE,
                FILE_NON_DIRECTORY_FILE,
                ptr::null_mut(),
            )
            .unwrap();
        file.write_all(b"checked metadata rights").unwrap();
        file.sync_all().unwrap();
        assert_eq!(file_info(file.as_raw_handle()).unwrap().nNumberOfLinks, 1);
        drop(file);
        drop(acl_handle);
        drop(parent);
        drop(directory);
        std::fs::remove_dir_all(root).unwrap();
    }
}
