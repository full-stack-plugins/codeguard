//! 安装服务的有界、校验后且不覆盖的工具字节发布层。
use crate::{install_stage::InstallStage, installed_artifact::InstalledArtifact};
use ring::digest::{SHA256, digest};
use std::{
    ffi::CString,
    fs::{File, OpenOptions, Permissions},
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
const MAX_BYTES: usize = 128 * 1024 * 1024;
/// 将冻结字节发布到已存在的受管缓存，校验后才出现最终名字。
/// 参数包含调用方固定的 SHA-256、剩余截止时间和取消标记；返回相对收据。
/// 此层不批准来源，不联网或执行；正式安装调用方必须先核验授权、平台及包格式。
pub fn publish_tool_bytes(
    cache_root: &Path,
    bytes: &[u8],
    expected_sha256: [u8; 32],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<InstalledArtifact, &'static str> {
    check_budget(deadline, cancelled)?;
    if !cache_root.is_absolute()
        || bytes.is_empty()
        || bytes.len() > MAX_BYTES
        || expected_sha256 == [0; 32]
    {
        return Err("install_input_invalid");
    }
    if digest(&SHA256, bytes).as_ref() != expected_sha256 {
        return Err("install_digest_mismatch");
    }
    check_budget(deadline, cancelled)?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(cache_root)
        .map_err(|_| "install_cache_unavailable")?;
    let metadata = directory
        .metadata()
        .map_err(|_| "install_cache_unavailable")?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Err("install_cache_permissions_invalid");
    }
    let hex: String = expected_sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let relative_path = format!("{hex}.bin");
    let destination = CString::new(relative_path.as_str()).map_err(|_| "install_input_invalid")?;
    if existing_matches(&directory, &destination, bytes.len(), &expected_sha256)? {
        check_budget(deadline, cancelled)?;
        return Ok(InstalledArtifact {
            relative_path,
            newly_published: false,
            byte_len: bytes.len() as u64,
        });
    }
    let mut stage = InstallStage::create(&directory)?;
    stage
        .file
        .set_permissions(Permissions::from_mode(0o700))
        .map_err(|_| "install_stage_write_failed")?;
    for chunk in bytes.chunks(128 * 1024) {
        check_budget(deadline, cancelled)?;
        stage
            .file
            .write_all(chunk)
            .map_err(|_| "install_stage_write_failed")?;
    }
    stage
        .file
        .sync_all()
        .map_err(|_| "install_stage_sync_failed")?;
    stage
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|_| "install_stage_verification_failed")?;
    let mut staged = Vec::new();
    Read::by_ref(&mut stage.file)
        .take((MAX_BYTES as u64) + 1)
        .read_to_end(&mut staged)
        .map_err(|_| "install_stage_verification_failed")?;
    let staged_metadata = stage
        .file
        .metadata()
        .map_err(|_| "install_stage_verification_failed")?;
    if staged.len() != bytes.len()
        || digest(&SHA256, &staged).as_ref() != expected_sha256
        || !staged_metadata.is_file()
        || staged_metadata.uid() != unsafe { libc::geteuid() }
        || staged_metadata.mode() & 0o777 != 0o700
        || staged_metadata.nlink() != 1
    {
        return Err("install_stage_verification_failed");
    }
    check_budget(deadline, cancelled)?;
    // 同一文件系统 linkat 原子新增且不覆盖；发布前最终目标不存在。
    let published = unsafe {
        libc::linkat(
            directory.as_raw_fd(),
            stage.name.as_ptr(),
            directory.as_raw_fd(),
            destination.as_ptr(),
            0,
        )
    } == 0;
    if !published {
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err("install_publish_failed");
        }
        if !existing_matches(&directory, &destination, bytes.len(), &expected_sha256)? {
            return Err("install_publish_failed");
        }
    }
    drop(stage);
    directory
        .sync_all()
        .map_err(|_| "install_cache_sync_failed")?;
    if !existing_matches(&directory, &destination, bytes.len(), &expected_sha256)? {
        return Err("installed_artifact_invalid");
    }
    check_budget(deadline, cancelled)?;
    Ok(InstalledArtifact {
        relative_path,
        newly_published: published,
        byte_len: bytes.len() as u64,
    })
}
fn check_budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("install_cancelled")
    } else if Instant::now() >= deadline {
        Err("install_deadline_exceeded")
    } else {
        Ok(())
    }
}
fn existing_matches(
    directory: &File,
    name: &CString,
    expected_len: usize,
    expected: &[u8; 32],
) -> Result<bool, &'static str> {
    // O_NONBLOCK 避免恶意 FIFO 使检查无限等待；先检查文件类型再读内容。
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(false)
        } else {
            Err("installed_artifact_invalid")
        };
    }
    // fd 在此转交 File 独占关闭。
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| "installed_artifact_invalid")?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.nlink() != 1
        || metadata.mode() & 0o777 != 0o700
        || metadata.len() != expected_len as u64
    {
        return Err("installed_artifact_invalid");
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "installed_artifact_invalid")?;
    if bytes.len() != expected_len || digest(&SHA256, &bytes).as_ref() != expected {
        return Err("installed_artifact_invalid");
    }
    Ok(true)
}
