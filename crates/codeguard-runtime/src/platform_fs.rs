//! Platform file primitives. Windows privacy uses a protected current-user DACL,
//! never a Unix-mode emulation. Handles reject reparse points before mutation.
#[cfg(unix)]
use std::fs;
use std::{fs::File, io, path::Path};

#[cfg(unix)]
pub fn set_private_permissions(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}
#[cfg(unix)]
pub fn verify_private_permissions(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if fs::symlink_metadata(path)?.permissions().mode() & 0o777 == mode {
        Ok(())
    } else {
        Err(io::Error::other("private mode mismatch"))
    }
}
#[cfg(unix)]
pub fn create_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path)
}
#[cfg(unix)]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}
#[cfg(unix)]
pub fn file_identity(path: &Path) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let m = fs::symlink_metadata(path)?;
    Ok((m.dev(), m.ino()))
}
#[cfg(unix)]
pub fn single_link(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.nlink() == 1)
}
#[cfg(unix)]
pub fn secure_create_new(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}
#[cfg(unix)]
pub fn secure_read(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(io::Error::other("not a single-link regular file"));
    }
    Ok(file)
}
#[cfg(unix)]
pub fn sync_parent(path: &Path) -> io::Result<()> {
    File::open(
        path.parent()
            .ok_or_else(|| io::Error::other("missing parent"))?,
    )?
    .sync_all()
}
#[cfg(unix)]
pub fn atomic_replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)?;
    sync_parent(to)
}
#[cfg(windows)]
pub use windows::*;

#[cfg(windows)]
mod windows {
    use super::*;
    use crate::windows_path::PinnedDirectory;
    use std::{
        ffi::OsStr,
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
        ptr,
    };
    use windows_sys::Wdk::Storage::FileSystem::{
        FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_WRITE_THROUGH,
    };
    use windows_sys::Win32::{
        Foundation::*,
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::Threading::*,
    };
    fn wide(s: &OsStr) -> io::Result<Vec<u16>> {
        let mut v: Vec<_> = s.encode_wide().collect();
        if v.contains(&0) {
            return Err(io::Error::other("NUL in path"));
        }
        v.push(0);
        Ok(v)
    }
    struct Descriptor(PSECURITY_DESCRIPTOR);
    impl Drop for Descriptor {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
    fn private_descriptor() -> io::Result<Descriptor> {
        unsafe {
            let mut token = ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut size = 0;
            GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut size);
            let mut buffer = vec![
                0usize;
                (size as usize + std::mem::size_of::<usize>() - 1)
                    / std::mem::size_of::<usize>()
            ];
            let ok = GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                size,
                &mut size,
            );
            CloseHandle(token);
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
            let mut sid = ptr::null_mut();
            if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut len = 0;
            while *sid.add(len) != 0 {
                len += 1;
            }
            let sid_string = String::from_utf16_lossy(std::slice::from_raw_parts(sid, len));
            LocalFree(sid.cast());
            // Protected DACL; child files/directories inherit only this user's full access.
            let sddl = wide(OsStr::new(&format!("D:P(A;OICI;FA;;;{sid_string})")))?;
            let mut descriptor = ptr::null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(Descriptor(descriptor))
        }
    }
    fn info(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
        let mut info = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::other("reparse point rejected"));
        }
        Ok(info)
    }
    fn pin_parent(path: &Path) -> io::Result<crate::windows_path::PinnedDirectory> {
        crate::windows_path::PinnedDirectory::open(
            path.parent()
                .ok_or_else(|| io::Error::other("parent required"))?,
        )
    }
    fn leaf(path: &Path) -> io::Result<&OsStr> {
        path.file_name()
            .ok_or_else(|| io::Error::other("ordinary file name required"))
    }
    fn open(path: &Path, access: u32) -> io::Result<File> {
        let parent = pin_parent(path)?;
        let file = parent.open_child(
            leaf(path)?,
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            0,
            ptr::null_mut(),
        )?;
        info(&file)?;
        Ok(file)
    }
    pub fn set_private_permissions(path: &Path, mode: u32) -> io::Result<()> {
        if mode != 0o600 && mode != 0o700 {
            return Err(io::Error::other("unsupported private mode"));
        }
        let file = open(path, WRITE_DAC | READ_CONTROL)?;
        let info = info(&file)?;
        if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0 && info.nNumberOfLinks != 1 {
            return Err(io::Error::other("hard link rejected"));
        }
        let descriptor = private_descriptor()?;
        let mut present = 0;
        let mut defaulted = 0;
        let mut acl = ptr::null_mut();
        if unsafe {
            GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut acl, &mut defaulted)
        } == 0
            || present == 0
            || acl.is_null()
        {
            return Err(io::Error::other("private DACL unavailable"));
        }
        let status = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                acl,
                ptr::null(),
            )
        };
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    }
    pub fn verify_private_permissions(path: &Path, mode: u32) -> io::Result<()> {
        if mode != 0o600 && mode != 0o700 {
            return Err(io::Error::other("unsupported private mode"));
        }
        let file = open(path, READ_CONTROL)?;
        let expected = private_descriptor()?;
        unsafe {
            let mut actual = ptr::null_mut();
            let mut actual_acl = ptr::null_mut();
            let status = GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut actual_acl,
                ptr::null_mut(),
                &mut actual,
            );
            if status != 0 {
                return Err(io::Error::from_raw_os_error(status as i32));
            }
            let actual = Descriptor(actual);
            let mut control = 0;
            let mut revision = 0;
            if GetSecurityDescriptorControl(actual.0, &mut control, &mut revision) == 0
                || control & SE_DACL_PROTECTED == 0
            {
                return Err(io::Error::other("DACL is not protected"));
            }
            let mut present = 0;
            let mut defaulted = 0;
            let mut expected_acl = ptr::null_mut();
            if GetSecurityDescriptorDacl(
                expected.0,
                &mut present,
                &mut expected_acl,
                &mut defaulted,
            ) == 0
                || actual_acl.is_null()
                || expected_acl.is_null()
            {
                return Err(io::Error::other("DACL unavailable"));
            }
            let actual_bytes =
                std::slice::from_raw_parts(actual_acl.cast::<u8>(), (*actual_acl).AclSize as usize);
            let expected_bytes = std::slice::from_raw_parts(
                expected_acl.cast::<u8>(),
                (*expected_acl).AclSize as usize,
            );
            if actual_bytes != expected_bytes {
                return Err(io::Error::other("private DACL mismatch"));
            }
        }
        Ok(())
    }
    pub fn create_private_dir(path: &Path) -> io::Result<()> {
        let parent = pin_parent(path)?;
        let descriptor = private_descriptor()?;
        let file = parent.open_child(
            leaf(path)?,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_CREATE,
            FILE_DIRECTORY_FILE,
            descriptor.0,
        )?;
        drop(file);
        Ok(())
    }
    pub fn secure_create_new(path: &Path) -> io::Result<File> {
        let parent = pin_parent(path)?;
        let descriptor = private_descriptor()?;
        parent.open_child(
            leaf(path)?,
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_DELETE,
            FILE_CREATE,
            FILE_NON_DIRECTORY_FILE,
            descriptor.0,
        )
    }
    pub fn file_identity(path: &Path) -> io::Result<(u64, u64)> {
        let file = open(path, FILE_READ_ATTRIBUTES)?;
        let i = info(&file)?;
        Ok((
            i.dwVolumeSerialNumber as u64,
            ((i.nFileIndexHigh as u64) << 32) | i.nFileIndexLow as u64,
        ))
    }
    pub fn single_link(path: &Path) -> bool {
        open(path, FILE_READ_ATTRIBUTES)
            .and_then(|f| info(&f))
            .is_ok_and(|i| {
                i.nNumberOfLinks == 1 && i.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            })
    }
    pub fn is_executable(path: &Path) -> bool {
        // Only native executables: batch files need an explicit shell contract.
        path.extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
            && open(path, GENERIC_READ)
                .and_then(|f| info(&f))
                .is_ok_and(|i| i.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0)
    }
    pub fn secure_read(path: &Path) -> io::Result<File> {
        let parent = pin_parent(path)?;
        let file = parent.open_child(
            leaf(path)?,
            GENERIC_READ,
            FILE_SHARE_READ,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE,
            ptr::null_mut(),
        )?;
        let i = info(&file)?;
        if i.nNumberOfLinks != 1 {
            return Err(io::Error::other("not a single-link regular file"));
        }
        Ok(file)
    }
    pub fn sync_parent(path: &Path) -> io::Result<()> {
        // Windows does not support FlushFileBuffers on directory handles. Re-open
        // the committed file for write-through flush. Handle-bound replacement
        // separately flushes its file; neither claims POSIX directory durability.
        open(path, GENERIC_WRITE)?.sync_all()
    }
    pub fn atomic_replace(from: &Path, to: &Path) -> io::Result<()> {
        let source_parent = pin_parent(from)?;
        let target_parent = PinnedDirectory::open_with_access(
            to.parent()
                .ok_or_else(|| io::Error::other("target parent required"))?,
            FILE_ADD_FILE,
        )?;
        let source = source_parent.open_child(
            leaf(from)?,
            DELETE | GENERIC_WRITE,
            FILE_SHARE_READ,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE | FILE_WRITE_THROUGH,
            ptr::null_mut(),
        )?;
        let i = info(&source)?;
        if i.nNumberOfLinks != 1 {
            return Err(io::Error::other("invalid replacement source"));
        }
        // Check the existing destination through the pinned parent. The rename
        // replaces that directory entry; it never follows a destination link.
        match target_parent.open_child(
            leaf(to)?,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            FILE_NON_DIRECTORY_FILE,
            ptr::null_mut(),
        ) {
            Ok(existing) => {
                if info(&existing)?.nNumberOfLinks != 1 {
                    return Err(io::Error::other("invalid replacement target"));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        rename_bound(&source, &target_parent, leaf(to)?, true)?;
        source.sync_all()
    }
    pub(crate) fn rename_bound(
        source: &File,
        target_parent: &PinnedDirectory,
        target_name: &OsStr,
        replace: bool,
    ) -> io::Result<()> {
        if std::path::Path::new(target_name).components().count() != 1
            || !matches!(
                std::path::Path::new(target_name).components().next(),
                Some(std::path::Component::Normal(_))
            )
            || target_name.to_string_lossy().contains(':')
        {
            return Err(io::Error::other("ordinary relative rename name required"));
        }
        let name = wide(target_name)?;
        let bytes = (name.len() - 1) * 2;
        let size = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + bytes;
        let mut buffer =
            vec![0usize; (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>()];
        unsafe {
            let rename = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
            (*rename).Anonymous.ReplaceIfExists = replace;
            (*rename).RootDirectory = target_parent.handle();
            (*rename).FileNameLength = bytes as u32;
            ptr::copy_nonoverlapping(
                name.as_ptr(),
                (*rename).FileName.as_mut_ptr(),
                name.len() - 1,
            );
            if SetFileInformationByHandle(
                source.as_raw_handle(),
                FileRenameInfo,
                rename.cast(),
                size as u32,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}
