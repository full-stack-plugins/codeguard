//! 完整工具树的有界私有暂存与不覆盖原子目录发布。
use crate::bundle_cache_fs::{budget, children, name, open_dir, parent, private};
use crate::bundle_install_stage::BundleInstallStage;
use crate::{InstalledBundle, UnpackedArchive, verify_unpacked_bundle_tree};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
/// 将完整已匹配工具树发布为单个内容寻址目录，失败不覆盖已有目录。
/// 参数为已存在缓存、冻结树、预期摘要及共享预算；返回经重验的相对定位。
/// 不下载/执行/批准来源，调用方必须独立绑定清单、平台和工具准备。
pub fn publish_tool_bundle(
    cache_root: &Path,
    tree: &UnpackedArchive,
    expected: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<InstalledBundle, &'static str> {
    budget(deadline, cancelled)?;
    if !cache_root.is_absolute() {
        return Err("bundle_install_cache_invalid");
    }
    verify_unpacked_bundle_tree(tree, expected, deadline, cancelled)?;
    let cache = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(cache_root)
        .map_err(|_| "bundle_install_cache_invalid")?;
    let m = cache
        .metadata()
        .map_err(|_| "bundle_install_cache_invalid")?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err("bundle_install_cache_permissions_invalid");
    }
    let relative_path = format!("{expected}.bundle");
    let destination = name(&relative_path)?;
    let byte_len = tree.files.iter().map(|f| f.bytes.len() as u64).sum();
    if exists(&cache, &relative_path, tree, deadline, cancelled)? {
        return Ok(InstalledBundle {
            relative_path,
            newly_published: false,
            byte_len,
        });
    }
    let mut stage = BundleInstallStage::create(&cache)?;
    stage
        .directory
        .set_permissions(Permissions::from_mode(0o700))
        .map_err(|_| "bundle_install_stage_write_failed")?;
    let mut dirs: Vec<_> = tree.directories.iter().collect();
    dirs.sort();
    for path in dirs {
        budget(deadline, cancelled)?;
        let (fd, leaf) = parent(&stage.directory, path)?;
        if unsafe { libc::mkdirat(fd.as_raw_fd(), leaf.as_ptr(), 0o700) } != 0 {
            return Err("bundle_install_stage_write_failed");
        }
        stage.directories.push(path.clone());
        open_dir(
            &fd,
            leaf.to_str().map_err(|_| "bundle_install_path_invalid")?,
        )?
        .set_permissions(Permissions::from_mode(0o700))
        .map_err(|_| "bundle_install_stage_write_failed")?;
    }
    for input in &tree.files {
        budget(deadline, cancelled)?;
        let (fd, leaf) = parent(&stage.directory, &input.relative_path)?;
        let raw = unsafe {
            libc::openat(
                fd.as_raw_fd(),
                leaf.as_ptr(),
                libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_WRONLY,
                0o700,
            )
        };
        if raw < 0 {
            return Err("bundle_install_stage_write_failed");
        }
        let mut file = unsafe { File::from_raw_fd(raw) };
        stage.files.push(input.relative_path.clone());
        file.set_permissions(Permissions::from_mode(0o700))
            .map_err(|_| "bundle_install_stage_write_failed")?;
        for chunk in input.bytes.chunks(128 * 1024) {
            budget(deadline, cancelled)?;
            file.write_all(chunk)
                .map_err(|_| "bundle_install_stage_write_failed")?;
        }
        file.sync_all()
            .map_err(|_| "bundle_install_stage_sync_failed")?;
    }
    for path in stage.directories.iter().rev() {
        let (fd, leaf) = parent(&stage.directory, path)?;
        open_dir(
            &fd,
            leaf.to_str().map_err(|_| "bundle_install_path_invalid")?,
        )?
        .sync_all()
        .map_err(|_| "bundle_install_stage_sync_failed")?;
    }
    stage
        .directory
        .sync_all()
        .map_err(|_| "bundle_install_stage_sync_failed")?;
    verify(&stage.directory, tree, deadline, cancelled)?;
    budget(deadline, cancelled)?;
    // 两个平台均使用原生独占重命名；不退回可能覆盖目录的普通 rename。
    #[cfg(target_os = "macos")]
    let renamed = unsafe {
        libc::renameatx_np(
            cache.as_raw_fd(),
            stage.name.as_ptr(),
            cache.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let renamed = unsafe {
        libc::renameat2(
            cache.as_raw_fd(),
            stage.name.as_ptr(),
            cache.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    let newly_published = renamed == 0;
    if newly_published {
        stage.published = true;
    } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
        return Err("bundle_install_publish_failed");
    }
    drop(stage);
    cache
        .sync_all()
        .map_err(|_| "bundle_install_cache_sync_failed")?;
    if !exists(&cache, &relative_path, tree, deadline, cancelled)? {
        return Err("bundle_installed_tree_invalid");
    }
    budget(deadline, cancelled)?;
    Ok(InstalledBundle {
        relative_path,
        newly_published,
        byte_len,
    })
}
fn exists(
    cache: &File,
    value: &str,
    tree: &UnpackedArchive,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<bool, &'static str> {
    let leaf = name(value)?;
    let raw = unsafe {
        libc::openat(
            cache.as_raw_fd(),
            leaf.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if raw < 0 {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(false)
        } else {
            Err("bundle_installed_tree_invalid")
        };
    }
    let directory = unsafe { File::from_raw_fd(raw) };
    verify(&directory, tree, deadline, cancelled)?;
    Ok(true)
}
fn verify(
    root: &File,
    tree: &UnpackedArchive,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<(), &'static str> {
    budget(deadline, cancelled)?;
    private(root, true)?;
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    members.insert(String::new(), Vec::new());
    for path in &tree.directories {
        members.entry(path.clone()).or_default();
    }
    for path in tree
        .directories
        .iter()
        .map(String::as_str)
        .chain(tree.files.iter().map(|f| f.relative_path.as_str()))
    {
        let (parent, leaf) = path.rsplit_once('/').unwrap_or(("", path));
        members
            .get_mut(parent)
            .ok_or("bundle_installed_tree_invalid")?
            .push(leaf.to_owned());
    }
    for (path, mut expected) in members {
        budget(deadline, cancelled)?;
        let directory = if path.is_empty() {
            root.try_clone()
                .map_err(|_| "bundle_installed_tree_invalid")?
        } else {
            let (fd, leaf) = parent(root, &path)?;
            open_dir(
                &fd,
                leaf.to_str().map_err(|_| "bundle_install_path_invalid")?,
            )?
        };
        private(&directory, true)?;
        expected.sort();
        if children(&directory, deadline, cancelled)? != expected {
            return Err("bundle_installed_tree_invalid");
        }
    }
    for input in &tree.files {
        budget(deadline, cancelled)?;
        let (fd, leaf) = parent(root, &input.relative_path)?;
        let raw = unsafe {
            libc::openat(
                fd.as_raw_fd(),
                leaf.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if raw < 0 {
            return Err("bundle_installed_tree_invalid");
        }
        let mut file = unsafe { File::from_raw_fd(raw) };
        private(&file, false)?;
        if file
            .metadata()
            .map_err(|_| "bundle_installed_tree_invalid")?
            .len()
            != input.bytes.len() as u64
        {
            return Err("bundle_installed_tree_invalid");
        }
        let mut buffer = vec![0; 128 * 1024];
        for chunk in input.bytes.chunks(buffer.len()) {
            budget(deadline, cancelled)?;
            file.read_exact(&mut buffer[..chunk.len()])
                .map_err(|_| "bundle_installed_tree_invalid")?;
            if buffer[..chunk.len()] != *chunk {
                return Err("bundle_installed_tree_invalid");
            }
        }
        let mut tail = [0];
        if file
            .read(&mut tail)
            .map_err(|_| "bundle_installed_tree_invalid")?
            != 0
        {
            return Err("bundle_installed_tree_invalid");
        }
        private(&file, false)?;
    }
    budget(deadline, cancelled)
}
