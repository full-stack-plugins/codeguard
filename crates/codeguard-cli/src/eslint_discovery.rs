//! ESLint 配置候选的只读发现，不执行 JS、不推断原生生效规则。
use crate::discovery::DiscoveryReport;
use codeguard_adapters::CheckerConfiguration;
use codeguard_core::ObservationPort;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) const ESLINT_CONFIG_NAMES: &[&str] = &[
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "eslint.config.ts",
    "eslint.config.mts",
    "eslint.config.cts",
    ".eslintrc",
    ".eslintrc.js",
    ".eslintrc.cjs",
    ".eslintrc.json",
    ".eslintrc.yaml",
    ".eslintrc.yml",
];

pub(crate) fn inspect_node_eslint<P: ObservationPort>(
    root: &Path,
    observation: &P,
    report: &mut DiscoveryReport,
    candidates: &BTreeSet<String>,
) {
    let mut by_root: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for candidate in candidates {
        by_root
            .entry(parent(candidate))
            .or_default()
            .push(candidate.clone());
    }
    let manifests: Vec<_> = report
        .manifest_sha256
        .iter()
        .filter(|(path, _)| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name == "package.json")
        })
        .map(|(path, digest)| (path.clone(), digest.clone()))
        .collect();
    if manifests.is_empty() && !by_root.contains_key(".") {
        if let Some(source) = report
            .languages
            .values()
            .flat_map(|language| &language.source_files)
            .filter(|source| !candidates.contains(*source))
            .find(|source| {
                ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]
                    .iter()
                    .any(|extension| {
                        Path::new(source)
                            .extension()
                            .is_some_and(|value| value == *extension)
                    })
            })
        {
            report.checker_configurations.push(entry(
                ".",
                source,
                "eslint_configuration_not_observed",
            ));
        }
    }
    for (manifest, digest) in manifests {
        let build_root = parent(&manifest);
        let files = by_root.entry(build_root.clone()).or_default();
        let reason = match observation.read_bounded(&root.join(&manifest), 256 * 1024) {
            Ok(bytes) if format!("{:x}", Sha256::digest(&bytes)) == digest => {
                match serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Ok(value)
                        if value
                            .as_object()
                            .is_some_and(|object| object.contains_key("eslintConfig")) =>
                    {
                        "eslint_legacy_configuration_requires_tool_context"
                    }
                    Ok(value) if value.is_object() => "eslint_configuration_not_observed",
                    _ => "eslint_manifest_invalid",
                }
            }
            _ => "eslint_manifest_input_unavailable_or_changed",
        };
        if reason == "eslint_manifest_input_unavailable_or_changed" {
            // 同轮输入失效必须使发现不完整，不能只隐藏为未知规则或允许画像刷新。
            report.observation_complete = false;
            report.blocked_paths.push(manifest.clone());
        }
        if files.is_empty()
            || matches!(
                reason,
                "eslint_manifest_input_unavailable_or_changed" | "eslint_manifest_invalid"
            )
        {
            report
                .checker_configurations
                .push(entry(&build_root, &manifest, reason));
        }
    }
    for (build_root, files) in by_root {
        for file in &files {
            let reason = if !report.checker_config_sha256.contains_key(file) {
                "eslint_config_unreadable"
            } else if files.len() > 1 {
                "eslint_config_selection_not_resolved"
            } else if Path::new(file)
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".eslintrc"))
            {
                "eslint_legacy_configuration_requires_tool_context"
            } else if file.ends_with(".ts") || file.ends_with(".mts") || file.ends_with(".cts") {
                "eslint_typescript_config_loader_not_verified"
            } else {
                "eslint_dynamic_configuration_not_evaluated"
            };
            report
                .checker_configurations
                .push(entry(&build_root, file, reason));
        }
    }
}
fn parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .filter(|path| !path.is_empty())
        .unwrap_or(".")
        .into()
}
fn entry(build_root: &str, source: &str, reason: &str) -> CheckerConfiguration {
    CheckerConfiguration {
        build_root:build_root.into(),checker_id:"node.eslint".into(),category:"lint".into(),configuration:"unknown".into(),configuration_ref:source.into(),reason:reason.into(),
        next_action:"确认项目原 ESLint 版本、调用参数及选中配置；用原生工具核验逐文件规则、parser/插件与导入闭包，不套用默认规则或自动迁移配置".into(),
    }
}
