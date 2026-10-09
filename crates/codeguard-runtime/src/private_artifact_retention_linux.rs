// Included in the Linux private store module to reuse its descriptor validation.
/// Exclusive lease for an explicit controller transaction. Never acquired by native commands.
pub struct RootLease<'a> {
    store: &'a PrivateArtifactStore,
    _lock: File,
}
impl PrivateArtifactStore {
    pub fn lease(&self) -> Result<RootLease<'_>, StoreError> {
        private_dir(&self.root)?;
        let lock = open_at(&self.root, c".", libc::O_RDONLY | libc::O_DIRECTORY)?;
        // SAFETY: the descriptor is owned and the lock is released when it is dropped.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(StoreError::Busy);
        }
        Ok(RootLease {
            store: self,
            _lock: lock,
        })
    }
}
fn control_name(name: &str) -> Result<CString, StoreError> {
    if !name.starts_with('.') || name.starts_with(".stage-") || name.starts_with(".control-stage-")
    {
        return Err(StoreError::InvalidInput);
    }
    cname(name)
}
fn same_slot(parent: &File, name: &CStr, file: &File) -> bool {
    let Ok(expected) = file.metadata() else {
        return false;
    };
    let mut observed = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: all pointers are live, output storage is initialized only on success.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
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
impl RootLease<'_> {
    pub fn is_empty(&self) -> Result<bool, StoreError> {
        private_dir(&self.store.root)?;
        Ok(entries(&self.store.root, 256)?.is_empty())
    }
    pub fn read_control(&self, name: &str, limit: u64) -> Result<Option<Vec<u8>>, StoreError> {
        let name = control_name(name)?;
        if limit > MAX_FILE_BYTES || limit > self.store.limits.max_total_bytes {
            return Err(StoreError::Quota);
        }
        private_dir(&self.store.root)?;
        // Distinguish genuine absence from unsafe symlinks/permissions without following links.
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.store.root.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(StoreError::Io)
            };
        }
        let mut file = open_at(&self.store.root, &name, libc::O_RDONLY)?;
        let size = private_file(&file, limit)?;
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
        if bytes.len() as u64 != size
            || private_file(&file, limit)? != size
            || !same_slot(&self.store.root, &name, &file)
        {
            return Err(StoreError::UnsafeEntry);
        }
        Ok(Some(bytes))
    }
    /// Atomic replacement of a private controller checkpoint; peak bytes count against quota.
    pub fn replace_control(&self, name: &str, bytes: &[u8], limit: u64) -> Result<(), StoreError> {
        self.replace_control_with_sync(name, bytes, limit, |root| root.sync_all())
    }
    fn replace_control_with_sync(
        &self,
        name: &str,
        bytes: &[u8],
        limit: u64,
        sync: impl FnOnce(&File) -> std::io::Result<()>,
    ) -> Result<(), StoreError> {
        let destination = control_name(name)?;
        if bytes.len() as u64 > limit || limit > MAX_FILE_BYTES {
            return Err(StoreError::Quota);
        }
        self.read_control(name, limit)?;
        let (count, current) = usage(&self.store.root, self.store.limits)?;
        if count >= self.store.limits.max_entries
            || current
                .checked_add(bytes.len() as u64)
                .is_none_or(|n| n > self.store.limits.max_total_bytes)
        {
            return Err(StoreError::Quota);
        }
        let temporary = cname(&format!(
            ".control-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))?;
        let raw = unsafe {
            libc::openat(
                self.store.root.as_raw_fd(),
                temporary.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        let mut file = opened(raw)?;
        let mut renamed = false;
        let result = (|| {
            if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                return Err(StoreError::Io);
            }
            private_file(&file, limit)?;
            file.write_all(bytes).map_err(io)?;
            file.sync_all().map_err(io)?;
            if !same_slot(&self.store.root, &temporary, &file) {
                return Err(StoreError::UnsafeEntry);
            }
            let mut actual = open_at(&self.store.root, &temporary, libc::O_RDONLY)?;
            if private_file(&actual, limit)? != bytes.len() as u64 {
                return Err(StoreError::UnsafeEntry);
            }
            let mut offset = 0;
            let mut buffer = [0; 8192];
            loop {
                let n = actual.read(&mut buffer).map_err(io)?;
                if n == 0 {
                    break;
                }
                if offset + n > bytes.len() || buffer[..n] != bytes[offset..offset + n] {
                    return Err(StoreError::UnsafeEntry);
                }
                offset += n;
            }
            if offset != bytes.len() {
                return Err(StoreError::UnsafeEntry);
            }
            self.read_control(name, limit)?;
            if unsafe {
                libc::renameat(
                    self.store.root.as_raw_fd(),
                    temporary.as_ptr(),
                    self.store.root.as_raw_fd(),
                    destination.as_ptr(),
                )
            } != 0
            {
                return Err(StoreError::Io);
            }
            renamed = true;
            sync(&self.store.root).map_err(|_| StoreError::PublicationIndeterminate)
        })();
        if !renamed && same_slot(&self.store.root, &temporary, &file) {
            unsafe {
                libc::unlinkat(self.store.root.as_raw_fd(), temporary.as_ptr(), 0);
            }
        }
        result
    }
    /// Validate every leaf before deletion; caller durably invalidates domain metadata first.
    pub fn purge(&self, destination: &str) -> Result<(), StoreError> {
        let destination = key(destination)?;
        private_dir(&self.store.root)?;
        let directory = open_at(
            &self.store.root,
            &destination,
            libc::O_RDONLY | libc::O_DIRECTORY,
        )?;
        private_dir(&directory)?;
        let names = entries(&directory, 8)?;
        let mut remaining = MAX_BUNDLE_BYTES.min(self.store.limits.max_total_bytes);
        let mut validated = Vec::new();
        for name in names {
            let name = cname(&name)?;
            let file = open_at(&directory, &name, libc::O_RDONLY)?;
            let size = private_file(&file, remaining.min(MAX_FILE_BYTES))?;
            remaining -= size;
            validated.push((name, file));
        }
        if !same_slot(&self.store.root, &destination, &directory) {
            return Err(StoreError::UnsafeEntry);
        }
        for (name, file) in &validated {
            if !same_slot(&directory, name, file) {
                return Err(StoreError::UnsafeEntry);
            }
            if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                return Err(StoreError::Io);
            }
        }
        directory.sync_all().map_err(io)?;
        if !same_slot(&self.store.root, &destination, &directory) {
            return Err(StoreError::UnsafeEntry);
        }
        if unsafe {
            libc::unlinkat(
                self.store.root.as_raw_fd(),
                destination.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } != 0
        {
            return Err(StoreError::Io);
        }
        self.store
            .root
            .sync_all()
            .map_err(|_| StoreError::PublicationIndeterminate)
    }
}
#[cfg(test)]
mod retention_faults {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn checkpoint_sync_failure_is_indeterminate_and_never_rolls_back_observed_bytes() {
        let path = std::env::temp_dir().join(format!(
            "cg-clock-fsync-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let store = PrivateArtifactStore::open(
            &path,
            StoreLimits {
                max_entries: 8,
                max_total_bytes: 4096,
            },
        )
        .unwrap();
        let lease = store.lease().unwrap();
        lease.replace_control(".clock", b"50", 64).unwrap();
        let error = lease.replace_control_with_sync(".clock", b"100", 64, |_| {
            File::open("/dev/null").unwrap().sync_all()
        });
        assert_eq!(error, Err(StoreError::PublicationIndeterminate));
        assert_eq!(lease.read_control(".clock", 64).unwrap().unwrap(), b"100");
        drop(lease);
        drop(store);
        let reopened = PrivateArtifactStore::open(
            &path,
            StoreLimits {
                max_entries: 8,
                max_total_bytes: 4096,
            },
        )
        .unwrap();
        assert_eq!(
            reopened
                .lease()
                .unwrap()
                .read_control(".clock", 64)
                .unwrap()
                .unwrap(),
            b"100"
        );
        std::fs::remove_dir_all(path).unwrap();
    }
}
