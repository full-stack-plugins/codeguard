//! 已绑定发行声明的脱敏布局投影，不读取包或形成批准。
use crate::distribution_artifact::DistributionArtifact;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
/// 生成声明路径关联摘要；只接受已由清单绑定服务核验的声明。
/// 返回固定阶段与动作，不显示包内私有路径、不执行内容核验或发布。
pub(crate) fn describe(artifact: &DistributionArtifact, manifest_version: &str) -> Value {
    let mut result = json!({"status":"missing_full_tree_declaration","install_tree_sha256":null,"cache_directory":null,"entrypoint_ref_sha256":null,"bundle_root_ref_sha256":null,"content_verification":"not_run","next_action":"bind_versioned_full_tree_layout"});
    if artifact.format == "raw" {
        result["status"] = "not_applicable_raw".into();
        result["next_action"] = if manifest_version == "1.2" {
            "verify_source_then_raw_package"
        } else {
            "bind_raw_cache_locator"
        }
        .into();
    } else if let Some(tree) = &artifact.install_tree_sha256 {
        let directory = format!("{tree}.bundle");
        result["status"] = "declared_paths_bound_untrusted".into();
        result["install_tree_sha256"] = tree.clone().into();
        result["cache_directory"] = directory.clone().into();
        result["entrypoint_ref_sha256"] = artifact.origin_ref_sha256.clone().into();
        if let Some(root) = &artifact.bundle_archive_root {
            let reference = if root.is_empty() {
                directory
            } else {
                format!("{directory}/{root}")
            };
            result["bundle_root_ref_sha256"] =
                format!("{:x}", Sha256::digest(reference.as_bytes())).into();
        }
        result["next_action"] = "verify_source_then_package_and_layout".into();
    }
    result
}
