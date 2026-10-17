//! RustSec 两类本地数据集合的有界内容/物理身份观察；稳定不等于可信来源或时效。
use codeguard_runtime::SourceSnapshot;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 固定集合内容及物理入口摘要；不包含根级Git或锁文件，不可作为漏洞库批准证据。
#[derive(Eq, PartialEq)]
pub(crate) struct CargoAuditDatabaseSnapshot {
    root_identity: (u64, u64),
    digest: [u8; 32],
}
impl CargoAuditDatabaseSnapshot {
    /// 按同轮截止时间与取消状态捕获crates/rust；返回不稳定、资源或读取阻塞。
    pub(crate) fn capture(
        root: &Path,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Self, &'static str> {
        check_budget(deadline, cancelled)?;
        let original =
            fs::symlink_metadata(root).map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
        if !original.is_dir()
            || original.file_type().is_symlink()
            || root.canonicalize().ok().as_deref() != Some(root)
        {
            return Err("cargo_audit_database_snapshot_unavailable");
        }
        let root_identity = (original.dev(), original.ino());
        let mut pending = vec![PathBuf::from("crates"), PathBuf::from("rust")];
        let mut entries = BTreeMap::new();
        let mut total_bytes = 0_u64;
        while let Some(relative) = pending.pop() {
            check_budget(deadline, cancelled)?;
            if entries.len() >= 16_384 || relative.components().count() > 16 {
                return Err("cargo_audit_database_snapshot_limit_exceeded");
            }
            let path = root.join(&relative);
            let metadata = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound
                        && relative.components().count() == 1 =>
                {
                    entries.insert(relative, (0_u8, 0_u64, 0_u64, 0_i64, 0_i64, [0_u8; 32]));
                    continue;
                }
                Err(_) => return Err("cargo_audit_database_snapshot_unavailable"),
            };
            if metadata.file_type().is_symlink() {
                return Err("cargo_audit_database_snapshot_unavailable");
            }
            let content = if metadata.is_dir() {
                let directory =
                    fs::read_dir(&path).map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
                for child in directory {
                    check_budget(deadline, cancelled)?;
                    if entries.len() + pending.len() >= 16_384 {
                        return Err("cargo_audit_database_snapshot_limit_exceeded");
                    }
                    let child = child.map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
                    pending.push(relative.join(child.file_name()));
                }
                [0_u8; 32]
            } else if metadata.is_file() {
                if metadata.len() > 2 * 1024 * 1024 {
                    return Err("cargo_audit_database_snapshot_limit_exceeded");
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or("cargo_audit_database_snapshot_limit_exceeded")?;
                if total_bytes > 64 * 1024 * 1024 {
                    return Err("cargo_audit_database_snapshot_limit_exceeded");
                }
                // 复用runtime的目录fd相对读取，禁止集合或父目录链接跳出选定根。
                let snapshot = SourceSnapshot::capture(
                    root,
                    [relative.clone()],
                    1,
                    2 * 1024 * 1024,
                    2 * 1024 * 1024,
                )
                .map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
                Sha256::digest(&snapshot.files()[&relative]).into()
            } else {
                return Err("cargo_audit_database_snapshot_unavailable");
            };
            let after = fs::symlink_metadata(&path)
                .map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
            if identity(&metadata) != identity(&after) || metadata.len() != after.len() {
                return Err("cargo_audit_database_changed");
            }
            entries.insert(
                relative,
                (
                    if metadata.is_dir() { 1 } else { 2 },
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                    content,
                ),
            );
        }
        check_budget(deadline, cancelled)?;
        let after =
            fs::symlink_metadata(root).map_err(|_| "cargo_audit_database_snapshot_unavailable")?;
        if !after.is_dir()
            || after.file_type().is_symlink()
            || (after.dev(), after.ino()) != root_identity
        {
            return Err("cargo_audit_database_changed");
        }
        let mut hash = Sha256::new();
        for (path, (kind, dev, ino, seconds, nanos, content)) in entries {
            let name = path.as_os_str().as_bytes();
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name);
            hash.update([kind]);
            hash.update(dev.to_le_bytes());
            hash.update(ino.to_le_bytes());
            hash.update(seconds.to_le_bytes());
            hash.update(nanos.to_le_bytes());
            hash.update(content);
        }
        Ok(Self {
            root_identity,
            digest: hash.finalize().into(),
        })
    }
}
fn identity(metadata: &fs::Metadata) -> (u64, u64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}
fn check_budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= deadline {
        Err("request_deadline_exceeded")
    } else {
        Ok(())
    }
}
