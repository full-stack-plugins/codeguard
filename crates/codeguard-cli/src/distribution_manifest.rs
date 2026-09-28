//! 发行清单的严格静态契约及与候选工具锁的精确关联。
use crate::{distribution_artifact::DistributionArtifact, tool_lock::parse_tool_lock_document};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
/// 未获信任的发行清单；结构解析和引用相符都不代表发行批准。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionManifest {
    /// 精确协议版本。
    pub schema_version: String,
    /// 清单稳定 ID。
    pub manifest_id: String,
    /// 工具锁原始字节的 SHA-256。
    pub lock_sha256: String,
    /// 显式声明的制品子集；不能证明全部工具/运行时覆盖。
    pub artifacts: Vec<DistributionArtifact>,
}
/// 从有界原始 JSON 解析清单，拒绝未知/重复字段和非法制品。
/// 返回静态结构或固定脱敏原因；不联网、执行或批准。
pub fn parse_distribution_manifest(bytes: &[u8]) -> Result<DistributionManifest, &'static str> {
    if bytes.len() > 256 * 1024 {
        return Err("distribution_manifest_too_large");
    }
    let manifest: DistributionManifest =
        serde_json::from_slice(bytes).map_err(|_| "distribution_manifest_invalid")?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}
/// 将每个声明关联到同一原始锁的精确受管制品；不是批准或完整安装证明。
/// 参数为已解析清单与锁字节；返回匹配的声明子集，缺失/跨目标均拒绝。
pub fn bind_distribution_manifest<'a>(
    manifest: &'a DistributionManifest,
    lock_bytes: &[u8],
) -> Result<Vec<&'a DistributionArtifact>, &'static str> {
    validate_manifest(manifest)?;
    if format!("{:x}", Sha256::digest(lock_bytes)) != manifest.lock_sha256 {
        return Err("distribution_lock_digest_mismatch");
    }
    let lock = parse_tool_lock_document(lock_bytes).map_err(|_| "distribution_lock_invalid")?;
    let mut bound = Vec::new();
    for artifact in &manifest.artifacts {
        let tool = lock
            .find(&artifact.tool_id, &artifact.platform)
            .ok_or("distribution_tool_unbound")?;
        if tool.origin_kind != "managed_cache"
            || tool.version != artifact.version
            || tool.binary_sha256 != artifact.binary_sha256
            || format!("{:x}", Sha256::digest(tool.origin_ref.as_bytes()))
                != artifact.origin_ref_sha256
            || tool.bundle.as_ref().map(|b| b.tree_sha256.as_str())
                != artifact.bundle_tree_sha256.as_deref()
        {
            return Err("distribution_tool_identity_mismatch");
        }
        if manifest.schema_version == "1.2"
            && artifact.format == "raw"
            && tool.origin_ref != format!("{}.bin", artifact.binary_sha256)
        {
            return Err("distribution_raw_locator_mismatch");
        }
        if let Some(tree) = &artifact.install_tree_sha256 {
            let slot = format!("{tree}.bundle");
            let entry = artifact
                .entrypoint
                .as_deref()
                .ok_or("distribution_install_layout_invalid")?;
            if tool.origin_ref != format!("{slot}/{entry}") {
                return Err("distribution_install_layout_mismatch");
            }
            if let Some(bundle) = &tool.bundle {
                let root = artifact
                    .bundle_archive_root
                    .as_deref()
                    .ok_or("distribution_install_layout_invalid")?;
                let expected = if root.is_empty() {
                    slot
                } else {
                    format!("{slot}/{root}")
                };
                if bundle.root != expected {
                    return Err("distribution_install_layout_mismatch");
                }
            }
        }
        bound.push(artifact);
    }
    Ok(bound)
}
fn validate_manifest(manifest: &DistributionManifest) -> Result<(), &'static str> {
    if !matches!(manifest.schema_version.as_str(), "1.0" | "1.1" | "1.2")
        || !token(&manifest.manifest_id)
        || !hash(&manifest.lock_sha256)
        || manifest.artifacts.is_empty()
        || manifest.artifacts.len() > 256
    {
        return Err("distribution_manifest_invalid");
    }
    let mut ids = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if manifest.schema_version == "1.0" && artifact.bundle_archive_root.is_some()
            || manifest.schema_version != "1.0"
                && (artifact.bundle_archive_root.is_some() != artifact.bundle_tree_sha256.is_some())
            || artifact
                .bundle_archive_root
                .as_deref()
                .is_some_and(|root| !root.is_empty() && !relative_entrypoint(root))
        {
            return Err("distribution_bundle_mapping_invalid");
        }
        if !artifact.install_tree_sha256.as_deref().is_none_or(hash)
            || (manifest.schema_version != "1.2" && artifact.install_tree_sha256.is_some())
            || (manifest.schema_version == "1.2"
                && artifact.format != "raw"
                && artifact.install_tree_sha256.is_none())
            || (artifact.format == "raw" && artifact.install_tree_sha256.is_some())
        {
            return Err("distribution_install_layout_invalid");
        }
        if !token(&artifact.tool_id)
            || artifact.version.is_empty()
            || artifact.version.len() > 128
            || artifact.version.chars().any(char::is_control)
            || !codeguard_core::CANDIDATE_PLATFORMS.contains(&artifact.platform.as_str())
            || !hash(&artifact.binary_sha256)
            || !hash(&artifact.origin_ref_sha256)
            || !hash(&artifact.package_sha256)
            || !artifact.bundle_tree_sha256.as_deref().is_none_or(hash)
            || !(1..=134217728).contains(&artifact.package_size_bytes)
            || !(1..=536870912).contains(&artifact.unpacked_size_limit_bytes)
            || !ids.insert((&artifact.tool_id, &artifact.platform))
        {
            return Err("distribution_artifact_invalid");
        }
        let uri: http::Uri = artifact
            .download_url
            .parse()
            .map_err(|_| "distribution_url_invalid")?;
        if artifact.download_url.len() > 2048
            || artifact.download_url.contains('#')
            || uri.scheme_str() != Some("https")
            || uri.query().is_some()
            || uri
                .authority()
                .is_none_or(|a| a.as_str().contains('@') || a.host().is_empty())
        {
            return Err("distribution_url_invalid");
        }
        match artifact.format.as_str() {
            "raw"
                if artifact.entrypoint.is_none()
                    && artifact.bundle_tree_sha256.is_none()
                    && artifact.bundle_archive_root.is_none()
                    && artifact.package_sha256 == artifact.binary_sha256
                    && artifact.unpacked_size_limit_bytes == artifact.package_size_bytes => {}
            "zip" | "tar_gz"
                if artifact
                    .entrypoint
                    .as_deref()
                    .is_some_and(relative_entrypoint) => {}
            _ => return Err("distribution_package_format_invalid"),
        }
    }
    Ok(())
}
fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b':'))
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        && value.bytes().any(|b| b != b'0')
}
fn relative_entrypoint(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}
