//! 初始化画像只恢复准备范围，不授权原生调用或沿用历史内容。
use codeguard_runtime::read_bounded_regular_file;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

/// 参数为物理工作区；返回当前清单不可用的历史构建根，或历史范围不可用原因。
pub(crate) fn collect(root: &Path) -> Result<Vec<String>, &'static str> {
    let Some(baseline) = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "historical_npm_workspace_invalid")?
    else {
        return Ok(Vec::new());
    };
    if baseline.workspace_id().is_none() {
        return Ok(Vec::new());
    }
    let path = root.join("codeguard/project.json");
    if path.canonicalize().ok().as_ref() != Some(&path) {
        return Err("historical_npm_profile_path_invalid");
    }
    let bytes = read_bounded_regular_file(&path, 4 * 1024 * 1024)
        .map_err(|_| "historical_npm_profile_unavailable")?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if baseline.managed_digest("codeguard/project.json") != Some(hash.as_str()) {
        return Err("historical_npm_profile_changed");
    }
    let profile = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "historical_npm_profile_invalid")?;
    if profile["schema_version"] != "0.3.0"
        || profile["document_type"] != "codeguard_project_profile"
        || profile["project_root"] != "."
        || profile["delivery_decision"] != "not_evaluated"
    {
        return Err("historical_npm_profile_invalid");
    }
    let manifests = profile["manifest_sha256"]
        .as_object()
        .filter(|m| m.len() <= 10_000)
        .ok_or("historical_npm_profile_invalid")?;
    let mut roots = BTreeSet::new();
    // 验证全部清单项后才返回范围，非法后续项不能留下半批历史根。
    for (manifest, digest) in manifests {
        if manifest.is_empty()
            || manifest
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || manifest.contains('\\')
            || manifest.contains(':')
            || manifest.chars().any(char::is_control)
            || Path::new(manifest)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !digest.as_str().is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err("historical_npm_profile_invalid");
        }
        if Path::new(manifest).file_name().and_then(|s| s.to_str()) != Some("package.json") {
            continue;
        }
        let parent = Path::new(manifest)
            .parent()
            .and_then(|p| p.to_str())
            .ok_or("historical_npm_profile_invalid")?;
        let build = if parent.is_empty() { "." } else { parent };
        let project = if build == "." {
            root.to_path_buf()
        } else {
            root.join(build)
        };
        if project.canonicalize().ok().as_ref() != Some(&project) || !project.is_dir() {
            return Err("historical_npm_project_unavailable");
        }
        if crate::npm_input_state::observe(&root.join(manifest), 256 * 1024).0 != "present" {
            roots.insert(build.to_owned());
        }
    }
    Ok(roots.into_iter().collect())
}
