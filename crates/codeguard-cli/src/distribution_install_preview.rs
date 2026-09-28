//! 安装清单的只读局部观察；不执行网络、发布或批准。
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 返回脱敏清单状态及绑定制品声明；参数为显式文件和本次读取的锁字节。
/// 缺失、非法或失配返回固定诊断，成功也只有未批准的子集绑定。
pub(crate) fn observe(path: Option<&Path>, lock: Option<&[u8]>) -> Value {
    let mut result = json!({"status":"not_provided","reason":"distribution_manifest_not_provided","sha256":null,"bound_artifacts":0,"artifacts":[]});
    let Some(path) = path else {
        return result;
    };
    let bytes = match read_bounded_regular_file(path, 256 * 1024) {
        Ok(bytes) => bytes,
        Err(_) => {
            result["status"] = "unreadable".into();
            result["reason"] = "distribution_manifest_unreadable".into();
            return result;
        }
    };
    result["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    let manifest = match parse_distribution_manifest(&bytes) {
        Ok(manifest) => manifest,
        Err(reason) => {
            result["status"] = "invalid".into();
            result["reason"] = reason.into();
            return result;
        }
    };
    let Some(lock) = lock else {
        result["status"] = "lock_unavailable".into();
        result["reason"] = "distribution_lock_unavailable".into();
        return result;
    };
    match bind_distribution_manifest(&manifest, lock) {
        Ok(artifacts) => {
            result["status"] = "bound_untrusted".into();
            result["reason"] = "distribution_authority_unverified".into();
            result["bound_artifacts"] = artifacts.len().into();
            result["artifacts"] = artifacts.iter().map(|a| json!({"tool_id":a.tool_id,"platform":a.platform,"format":a.format,"package_sha256":a.package_sha256,"package_size_bytes":a.package_size_bytes,"unpacked_size_limit_bytes":a.unpacked_size_limit_bytes,"source_uri_sha256":format!("{:x}",Sha256::digest(a.download_url.as_bytes())),"layout":crate::distribution_layout_preview::describe(a, &manifest.schema_version)})).collect::<Vec<_>>().into();
        }
        Err(reason) => {
            result["status"] = "unbound".into();
            result["reason"] = reason.into();
        }
    }
    result
}
