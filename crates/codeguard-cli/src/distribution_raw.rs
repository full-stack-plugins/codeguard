//! raw 制品与内容寻址锁定位的有界内容关联。
use crate::RawDistribution;
use crate::distribution_bundle::digest_bytes;
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
/// 核对新版清单、原工具锁、精确 raw 包长度/摘要及缓存定位。
/// 参数为清单/锁原字节、工具/平台、不可变包和共享预算；返回借用视图。
/// 不复制内容、写缓存、执行工具或授予来源批准。
pub fn inspect_distribution_raw<'a>(
    manifest_bytes: &[u8],
    lock_bytes: &[u8],
    tool_id: &str,
    platform: &str,
    package: &'a [u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<RawDistribution<'a>, &'static str> {
    budget(deadline, cancelled)?;
    let manifest = parse_distribution_manifest(manifest_bytes)?;
    let artifacts = bind_distribution_manifest(&manifest, lock_bytes)?;
    if manifest.schema_version != "1.2" {
        return Err("distribution_raw_mapping_missing");
    }
    let artifact = artifacts
        .into_iter()
        .find(|a| a.tool_id == tool_id && a.platform == platform)
        .ok_or("distribution_raw_artifact_missing")?;
    if artifact.format != "raw" {
        return Err("distribution_raw_format_required");
    }
    if package.len() as u64 != artifact.package_size_bytes {
        return Err("distribution_package_size_mismatch");
    }
    let expected = digest_bytes(&artifact.binary_sha256)?;
    let mut digest = Sha256::new();
    for chunk in package.chunks(128 * 1024) {
        budget(deadline, cancelled)?;
        digest.update(chunk);
    }
    if digest.finalize()[..] != expected {
        return Err("distribution_raw_digest_mismatch");
    }
    budget(deadline, cancelled)?;
    Ok(RawDistribution {
        bytes: package,
        binary_sha256: expected,
        origin_ref: format!("{}.bin", artifact.binary_sha256),
        lock_sha256: manifest.lock_sha256.clone(),
        manifest_sha256: format!("{:x}", Sha256::digest(manifest_bytes)),
    })
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Err("distribution_raw_cancelled")
    } else if Instant::now() >= deadline {
        Err("distribution_raw_deadline_exceeded")
    } else {
        Ok(())
    }
}
