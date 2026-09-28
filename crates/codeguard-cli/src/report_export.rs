//! 检查报告的同目录暂存与原子替换；导出失败不删除内存中的原生发现。

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_EXPORT: AtomicU64 = AtomicU64::new(0);

/// 将所选格式的完整字节原子写入目标；参数为输出路径和序列化内容，失败返回静态原因码。
pub fn export_report(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() || path.file_name().is_none() {
        return Err("output_parent_unavailable");
    }
    let prior = match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => return Err("output_not_regular_file"),
        Ok(_) => {
            let old = read_bounded(path)?;
            if !is_codeguard_report(&old) {
                return Err("output_existing_file_is_not_codeguard_report");
            }
            Some(old)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("output_identity_unavailable"),
    };
    let temp = parent.join(format!(
        ".codeguard-export-{}-{}.tmp",
        std::process::id(),
        NEXT_EXPORT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|_| "output_temp_create_failed")?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if written.is_err() {
        let _ = fs::remove_file(&temp);
        return Err("output_temp_write_failed");
    }
    let renamed = if let Some(old) = prior {
        if read_bounded(path).ok().as_deref() != Some(old.as_slice()) {
            Err(std::io::Error::other("output changed during export"))
        } else {
            fs::rename(&temp, path)
        }
    } else {
        fs::hard_link(&temp, path)
    };
    let _ = fs::remove_file(&temp);
    renamed.map_err(|_| "output_replace_failed")?;
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, &'static str> {
    let file = fs::File::open(path).map_err(|_| "output_existing_read_failed")?;
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "output_existing_read_failed")?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("output_existing_too_large");
    }
    Ok(bytes)
}

fn is_codeguard_report(bytes: &[u8]) -> bool {
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    matches!(
        document["report_type"].as_str(),
        Some("check_feedback" | "check_aborted")
    ) || (document["version"] == "2.1.0"
        && document["runs"][0]["tool"]["driver"]["name"] == "CodeGuard")
}
