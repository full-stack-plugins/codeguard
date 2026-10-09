//! Explicit Linux private immutable bundles. No execution, trust or current-pointer policy.
use std::{collections::BTreeMap, path::Path};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    Unsupported,
    InvalidRoot,
    InvalidInput,
    UnsafeEntry,
    Busy,
    Quota,
    Io,
    AlreadyExists,
    PublicationIndeterminate,
}
#[derive(Debug, Clone, Copy)]
pub struct StoreLimits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
}
pub struct ArtifactLimit<'a> {
    pub name: &'a str,
    pub max_bytes: u64,
}
pub struct ArtifactInput<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}
pub const MAX_BUNDLE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        },
        path::Component,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    pub struct PrivateArtifactStore {
        root: Arc<File>,
        limits: StoreLimits,
    }
    pub struct StagedArtifacts {
        root: Arc<File>,
        directory: File,
        _lock: File,
        stage: CString,
        destination: CString,
        leaves: Vec<CString>,
        renamed: bool,
    }
    fn io(_: std::io::Error) -> StoreError {
        StoreError::Io
    }
    fn cname(name: &str) -> Result<CString, StoreError> {
        if name.is_empty()
            || name.len() > 128
            || matches!(name, "." | "..")
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(StoreError::InvalidInput);
        }
        CString::new(name).map_err(|_| StoreError::InvalidInput)
    }
    fn key(key: &str) -> Result<CString, StoreError> {
        if key.len() != 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(StoreError::InvalidInput);
        }
        cname(key)
    }
    fn opened(raw: i32) -> Result<File, StoreError> {
        if raw < 0 {
            return Err(StoreError::Io);
        }
        // SAFETY: successful open/openat returns a new owned descriptor.
        Ok(unsafe { File::from_raw_fd(raw) })
    }
    fn open_at(parent: &File, name: &CStr, flags: i32) -> Result<File, StoreError> {
        // SAFETY: parent and nul-terminated name remain valid for this call.
        opened(unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        })
    }
    fn private_dir(file: &File) -> Result<(), StoreError> {
        let m = file.metadata().map_err(io)?;
        // SAFETY: geteuid has no preconditions.
        if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o7777 != 0o700 {
            return Err(StoreError::UnsafeEntry);
        }
        Ok(())
    }
    fn private_file(file: &File, limit: u64) -> Result<u64, StoreError> {
        let m = file.metadata().map_err(io)?;
        // SAFETY: geteuid has no preconditions.
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o7777 != 0o600
            || m.nlink() != 1
        {
            return Err(StoreError::UnsafeEntry);
        }
        if m.len() > limit {
            return Err(StoreError::Quota);
        }
        Ok(m.len())
    }
    fn explicit_root(path: &Path) -> Result<File, StoreError> {
        if !path.is_absolute() || path.as_os_str().as_bytes().len() > 4096 {
            return Err(StoreError::InvalidRoot);
        }
        // Walk all components without following any ancestor symlink.
        let mut root = File::open("/").map_err(io)?;
        let mut count = 0;
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(part) => {
                    count += 1;
                    if count > 64 {
                        return Err(StoreError::InvalidRoot);
                    }
                    let name =
                        CString::new(part.as_bytes()).map_err(|_| StoreError::InvalidRoot)?;
                    root = open_at(&root, &name, libc::O_RDONLY | libc::O_DIRECTORY)
                        .map_err(|_| StoreError::InvalidRoot)?;
                }
                _ => return Err(StoreError::InvalidRoot),
            }
        }
        private_dir(&root).map_err(|_| StoreError::InvalidRoot)?;
        Ok(root)
    }
    fn entries(directory: &File, limit: usize) -> Result<Vec<String>, StoreError> {
        let scan = open_at(directory, c".", libc::O_RDONLY | libc::O_DIRECTORY)?;
        use std::os::fd::IntoRawFd;
        let raw = scan.into_raw_fd();
        // SAFETY: ownership of this fresh descriptor transfers to fdopendir on success.
        let dir = unsafe { libc::fdopendir(raw) };
        if dir.is_null() {
            unsafe { libc::close(raw) };
            return Err(StoreError::Io);
        }
        struct Directory(*mut libc::DIR);
        impl Drop for Directory {
            fn drop(&mut self) {
                unsafe { libc::closedir(self.0) };
            }
        }
        let guard = Directory(dir);
        let mut result = Vec::new();
        loop {
            // SAFETY: guard owns the directory stream, no concurrent use.
            unsafe {
                *libc::__errno_location() = 0;
            }
            let entry = unsafe { libc::readdir(guard.0) };
            if entry.is_null() {
                if unsafe { *libc::__errno_location() } != 0 {
                    return Err(StoreError::Io);
                }
                break;
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            if result.len() >= limit {
                return Err(StoreError::Quota);
            }
            let value = std::str::from_utf8(bytes).map_err(|_| StoreError::UnsafeEntry)?;
            cname(value).map_err(|_| StoreError::UnsafeEntry)?;
            result.push(value.to_owned());
        }
        result.sort();
        Ok(result)
    }
    fn count_file(mut file: File, limit: u64) -> Result<u64, StoreError> {
        let expected = private_file(&file, limit.min(MAX_FILE_BYTES))?;
        let mut count = 0u64;
        let mut buffer = [0; 8192];
        loop {
            let n = file.read(&mut buffer).map_err(io)?;
            if n == 0 {
                break;
            }
            count = count.checked_add(n as u64).ok_or(StoreError::Quota)?;
            if count > expected {
                return Err(StoreError::Quota);
            }
        }
        if count != expected || private_file(&file, limit.min(MAX_FILE_BYTES))? != expected {
            return Err(StoreError::UnsafeEntry);
        }
        Ok(count)
    }
    fn usage(root: &File, limits: StoreLimits) -> Result<(usize, u64), StoreError> {
        let items = entries(root, limits.max_entries)?;
        let mut total = 0;
        for item in &items {
            let file = open_at(root, &cname(item)?, libc::O_RDONLY)?;
            if file.metadata().map_err(io)?.is_dir() {
                private_dir(&file)?;
                for leaf in entries(&file, 8)? {
                    let payload = open_at(&file, &cname(&leaf)?, libc::O_RDONLY)?;
                    total += count_file(payload, limits.max_total_bytes - total)?;
                }
            } else {
                total += count_file(file, limits.max_total_bytes - total)?;
            }
        }
        Ok((items.len(), total))
    }
    impl PrivateArtifactStore {
        pub fn open(root: &Path, limits: StoreLimits) -> Result<Self, StoreError> {
            if limits.max_entries == 0
                || limits.max_entries > 256
                || limits.max_total_bytes == 0
                || limits.max_total_bytes > 512 * 1024 * 1024
            {
                return Err(StoreError::InvalidInput);
            }
            Ok(Self {
                root: Arc::new(explicit_root(root)?),
                limits,
            })
        }
        pub fn stage(
            &self,
            destination: &str,
            inputs: &[ArtifactInput<'_>],
        ) -> Result<StagedArtifacts, StoreError> {
            self.stage_with(destination, inputs, |file, bytes| file.write_all(bytes))
        }
        fn stage_with(
            &self,
            destination: &str,
            inputs: &[ArtifactInput<'_>],
            mut write: impl FnMut(&mut File, &[u8]) -> std::io::Result<()>,
        ) -> Result<StagedArtifacts, StoreError> {
            let destination = key(destination)?;
            let mut incoming = 0u64;
            if inputs.is_empty() || inputs.len() > 8 {
                return Err(StoreError::InvalidInput);
            }
            for (i, input) in inputs.iter().enumerate() {
                cname(input.name)?;
                if input.bytes.len() as u64 > MAX_FILE_BYTES
                    || inputs[..i].iter().any(|other| other.name == input.name)
                {
                    return Err(StoreError::InvalidInput);
                }
                incoming = incoming
                    .checked_add(input.bytes.len() as u64)
                    .ok_or(StoreError::Quota)?;
            }
            if incoming > MAX_BUNDLE_BYTES || incoming > self.limits.max_total_bytes {
                return Err(StoreError::Quota);
            }
            private_dir(&self.root)?;
            // Fresh open description: independent simultaneous callers must contend too.
            let lock = open_at(&self.root, c".", libc::O_RDONLY | libc::O_DIRECTORY)?;
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(StoreError::Busy);
            }
            let (count, bytes) = usage(&self.root, self.limits)?;
            if count >= self.limits.max_entries
                || bytes
                    .checked_add(incoming)
                    .is_none_or(|sum| sum > self.limits.max_total_bytes)
            {
                return Err(StoreError::Quota);
            }
            let mut created = None;
            for _ in 0..16 {
                let stage = cname(&format!(
                    ".stage-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ))?;
                if unsafe { libc::mkdirat(self.root.as_raw_fd(), stage.as_ptr(), 0o700) } == 0 {
                    created = Some(stage);
                    break;
                }
                if std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(StoreError::Io);
                }
            }
            let stage = created.ok_or(StoreError::Busy)?;
            let directory = match open_at(&self.root, &stage, libc::O_RDONLY | libc::O_DIRECTORY) {
                Ok(file) => file,
                Err(error) => {
                    unsafe {
                        libc::unlinkat(self.root.as_raw_fd(), stage.as_ptr(), libc::AT_REMOVEDIR)
                    };
                    return Err(error);
                }
            };
            if unsafe { libc::fchmod(directory.as_raw_fd(), 0o700) } != 0 {
                unsafe {
                    libc::unlinkat(self.root.as_raw_fd(), stage.as_ptr(), libc::AT_REMOVEDIR)
                };
                return Err(StoreError::Io);
            }
            let mut result = StagedArtifacts {
                root: self.root.clone(),
                directory,
                _lock: lock,
                stage,
                destination,
                leaves: Vec::new(),
                renamed: false,
            };
            for input in inputs {
                let leaf = cname(input.name)?;
                let raw = unsafe {
                    libc::openat(
                        result.directory.as_raw_fd(),
                        leaf.as_ptr(),
                        libc::O_WRONLY
                            | libc::O_CREAT
                            | libc::O_EXCL
                            | libc::O_NOFOLLOW
                            | libc::O_CLOEXEC,
                        0o600,
                    )
                };
                let mut file = opened(raw)?;
                result.leaves.push(leaf);
                // Explicit private mode, unaffected by controller umask.
                if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                    return Err(StoreError::Io);
                }
                private_file(&file, MAX_FILE_BYTES)?;
                write(&mut file, input.bytes).map_err(io)?;
                file.sync_all().map_err(io)?;
                // Verify the actual staged descriptor bytes, not just the input serialization.
                let mut actual = open_at(
                    &result.directory,
                    result.leaves.last().ok_or(StoreError::Io)?,
                    libc::O_RDONLY,
                )?;
                if private_file(&actual, MAX_FILE_BYTES)? != input.bytes.len() as u64 {
                    return Err(StoreError::UnsafeEntry);
                }
                let mut offset = 0usize;
                let mut buffer = [0u8; 8192];
                loop {
                    let n = actual.read(&mut buffer).map_err(io)?;
                    if n == 0 {
                        break;
                    }
                    if offset + n > input.bytes.len()
                        || buffer[..n] != input.bytes[offset..offset + n]
                    {
                        return Err(StoreError::UnsafeEntry);
                    }
                    offset += n;
                }
                if offset != input.bytes.len()
                    || private_file(&actual, MAX_FILE_BYTES)? != input.bytes.len() as u64
                {
                    return Err(StoreError::UnsafeEntry);
                }
            }
            result.directory.sync_all().map_err(io)?;
            Ok(result)
        }
        pub fn read(&self, destination: &str) -> Result<BTreeMap<String, Vec<u8>>, StoreError> {
            self.read_inner(destination, None)
        }
        pub fn read_with_limits(
            &self,
            destination: &str,
            limits: &[ArtifactLimit<'_>],
        ) -> Result<BTreeMap<String, Vec<u8>>, StoreError> {
            if limits.is_empty() || limits.len() > 8 {
                return Err(StoreError::InvalidInput);
            }
            for limit in limits {
                cname(limit.name)?;
                if limit.max_bytes > MAX_FILE_BYTES {
                    return Err(StoreError::InvalidInput);
                }
            }
            self.read_inner(destination, Some(limits))
        }
        fn read_inner(
            &self,
            destination: &str,
            limits: Option<&[ArtifactLimit<'_>]>,
        ) -> Result<BTreeMap<String, Vec<u8>>, StoreError> {
            let destination = key(destination)?;
            private_dir(&self.root)?;
            let directory = open_at(&self.root, &destination, libc::O_RDONLY | libc::O_DIRECTORY)?;
            private_dir(&directory)?;
            let mut remaining = MAX_BUNDLE_BYTES.min(self.limits.max_total_bytes);
            let mut result = BTreeMap::new();
            for leaf in entries(&directory, 8)? {
                let limit = match limits {
                    Some(limits) => {
                        limits
                            .iter()
                            .find(|limit| limit.name == leaf)
                            .ok_or(StoreError::UnsafeEntry)?
                            .max_bytes
                    }
                    None => MAX_FILE_BYTES,
                };
                let mut file = open_at(&directory, &cname(&leaf)?, libc::O_RDONLY)?;
                let size = private_file(&file, remaining.min(limit))?;
                let mut bytes = Vec::with_capacity(size as usize);
                let mut buffer = [0; 8192];
                loop {
                    let n = file.read(&mut buffer).map_err(io)?;
                    if n == 0 {
                        break;
                    }
                    if bytes.len() + n > size as usize {
                        return Err(StoreError::Quota);
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
                if bytes.len() as u64 != size || private_file(&file, remaining.min(limit))? != size
                {
                    return Err(StoreError::UnsafeEntry);
                }
                remaining -= bytes.len() as u64;
                result.insert(leaf, bytes);
            }
            Ok(result)
        }
    }
    impl StagedArtifacts {
        fn owns_slot(&self) -> bool {
            let Ok(expected) = self.directory.metadata() else {
                return false;
            };
            let mut observed = std::mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe {
                libc::fstatat(
                    self.root.as_raw_fd(),
                    self.stage.as_ptr(),
                    observed.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                return false;
            }
            let observed = unsafe { observed.assume_init() };
            observed.st_dev == expected.dev() && observed.st_ino == expected.ino()
        }
        pub fn publish(self) -> Result<(), StoreError> {
            self.publish_with(|root| root.sync_all())
        }
        fn publish_with(
            mut self,
            sync: impl FnOnce(&File) -> std::io::Result<()>,
        ) -> Result<(), StoreError> {
            private_dir(&self.root)?;
            if !self.owns_slot() {
                return Err(StoreError::UnsafeEntry);
            }
            if unsafe {
                libc::renameat2(
                    self.root.as_raw_fd(),
                    self.stage.as_ptr(),
                    self.root.as_raw_fd(),
                    self.destination.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            } != 0
            {
                return Err(
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
                        StoreError::AlreadyExists
                    } else {
                        StoreError::Io
                    },
                );
            }
            self.renamed = true;
            sync(&self.root).map_err(|_| StoreError::PublicationIndeterminate)
        }
    }
    impl Drop for StagedArtifacts {
        fn drop(&mut self) {
            if !self.renamed {
                for leaf in &self.leaves {
                    unsafe { libc::unlinkat(self.directory.as_raw_fd(), leaf.as_ptr(), 0) };
                }
                if self.owns_slot() {
                    unsafe {
                        libc::unlinkat(
                            self.root.as_raw_fd(),
                            self.stage.as_ptr(),
                            libc::AT_REMOVEDIR,
                        )
                    };
                }
            }
        }
    }
    #[cfg(test)]
    mod faults {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        struct Root(std::path::PathBuf);
        impl Root {
            fn new() -> Self {
                let path = std::env::temp_dir().join(format!(
                    "cg-store-fault-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir(&path).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
                Self(path)
            }
        }
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        #[test]
        fn real_write_errors_remove_only_stage_and_preserve_existing_bytes() {
            for full in [true, false] {
                let root = Root::new();
                let store = PrivateArtifactStore::open(
                    &root.0,
                    StoreLimits {
                        max_entries: 8,
                        max_total_bytes: 1000,
                    },
                )
                .unwrap();
                let old = "a".repeat(64);
                store
                    .stage(
                        &old,
                        &[ArtifactInput {
                            name: "data",
                            bytes: b"old findings",
                        }],
                    )
                    .unwrap()
                    .publish()
                    .unwrap();
                let mut faulty = if full {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .open("/dev/full")
                        .unwrap()
                } else {
                    File::open(root.0.join(&old).join("data")).unwrap()
                };
                let mut observed = None;
                let result = store.stage_with(
                    &"b".repeat(64),
                    &[ArtifactInput {
                        name: "data",
                        bytes: b"new findings",
                    }],
                    |sink, bytes| {
                        sink.write_all(&bytes[..3])?;
                        let error = faulty.write_all(&bytes[3..]).unwrap_err();
                        observed = error.raw_os_error();
                        Err(error)
                    },
                );
                assert!(result.is_err());
                assert_eq!(
                    observed,
                    Some(if full { libc::ENOSPC } else { libc::EBADF })
                );
                assert_eq!(store.read(&old).unwrap()["data"], b"old findings");
                assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 1);
            }
        }
        #[test]
        fn actual_stage_bytes_must_match_even_if_sink_claims_success() {
            for wrong in [b"short".as_slice(), b"wrong bytes".as_slice()] {
                let root = Root::new();
                let store = PrivateArtifactStore::open(
                    &root.0,
                    StoreLimits {
                        max_entries: 8,
                        max_total_bytes: 1000,
                    },
                )
                .unwrap();
                assert!(
                    store
                        .stage_with(
                            &"a".repeat(64),
                            &[ArtifactInput {
                                name: "data",
                                bytes: b"exact bytes"
                            }],
                            |sink, _| sink.write_all(wrong)
                        )
                        .is_err()
                );
                assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 0);
            }
        }
        #[test]
        fn post_rename_real_fsync_error_is_indeterminate_and_never_overwrites() {
            let root = Root::new();
            let store = PrivateArtifactStore::open(
                &root.0,
                StoreLimits {
                    max_entries: 8,
                    max_total_bytes: 1000,
                },
            )
            .unwrap();
            let key = "a".repeat(64);
            let result = store
                .stage(
                    &key,
                    &[ArtifactInput {
                        name: "data",
                        bytes: b"published",
                    }],
                )
                .unwrap()
                .publish_with(|_| File::open("/dev/null")?.sync_all());
            assert_eq!(result, Err(StoreError::PublicationIndeterminate));
            assert_eq!(store.read(&key).unwrap()["data"], b"published");
            assert!(
                store
                    .stage(
                        &key,
                        &[ArtifactInput {
                            name: "data",
                            bytes: b"retry"
                        }]
                    )
                    .unwrap()
                    .publish()
                    .is_err()
            );
            assert_eq!(store.read(&key).unwrap()["data"], b"published");
        }
    }
}
#[cfg(target_os = "linux")]
pub use linux::{PrivateArtifactStore, StagedArtifacts};
#[cfg(not(target_os = "linux"))]
pub struct PrivateArtifactStore;
#[cfg(not(target_os = "linux"))]
pub struct StagedArtifacts;
#[cfg(not(target_os = "linux"))]
impl PrivateArtifactStore {
    pub fn open(_: &Path, _: StoreLimits) -> Result<Self, StoreError> {
        Err(StoreError::Unsupported)
    }
    pub fn stage(&self, _: &str, _: &[ArtifactInput<'_>]) -> Result<StagedArtifacts, StoreError> {
        Err(StoreError::Unsupported)
    }
    pub fn read(&self, _: &str) -> Result<BTreeMap<String, Vec<u8>>, StoreError> {
        Err(StoreError::Unsupported)
    }
    pub fn read_with_limits(
        &self,
        _: &str,
        _: &[ArtifactLimit<'_>],
    ) -> Result<BTreeMap<String, Vec<u8>>, StoreError> {
        Err(StoreError::Unsupported)
    }
}
#[cfg(not(target_os = "linux"))]
impl StagedArtifacts {
    pub fn publish(self) -> Result<(), StoreError> {
        Err(StoreError::Unsupported)
    }
}
