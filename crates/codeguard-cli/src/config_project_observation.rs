//! 配置命令的有界原生配置观察；复用发现服务，不执行工具或授予策略权威。
use crate::discovery::discover;
use codeguard_adapters::legacy_registry;
use codeguard_runtime::NativeObservation;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
const MAX_ROWS: usize = 256;
const MAX_ROOTS: usize = 64;
const MAX_REASONS: usize = 32;

/// 观察指定目录的原生配置声明、来源摘要和未解析条件，返回有界静态投影。
/// 空范围、截断或坏配置不成为检查通过；生效规则及原生抑制须另用原工具解析。
pub(crate) fn observe(root: &Path) -> Value {
    let mut result = json!({"basis":"static_project_discovery","observation_status":"incomplete","native_execution":"not_run","effective_rules":"unresolved","suppressions":"unresolved","truncated":false,
        "build_root_count":0,"build_roots":[],"manifest_count":0,"manifest_sha256":{},"checker_config_count":0,"checker_config_sha256":{},"checker_count":0,"checker_configurations":[],
        "blocked_path_count":0,"blocked_paths":[],"unknown_condition_count":1,"unknown_conditions":["bundled_registry_unavailable"]});
    let Ok(registry) = legacy_registry() else {
        return result;
    };
    let discovery = discover(root, &registry, &NativeObservation);
    let truncated = discovery.build_roots.len() > MAX_ROOTS
        || discovery.manifest_sha256.len() > MAX_ROWS
        || discovery.checker_config_sha256.len() > MAX_ROWS
        || discovery.checker_configurations.len() > MAX_ROWS
        || discovery.blocked_paths.len() > MAX_REASONS
        || discovery.unknown_conditions.len() > MAX_REASONS;
    result["observation_status"] = json!(if discovery.observation_complete && !truncated {
        "complete"
    } else {
        "incomplete"
    });
    result["truncated"] = json!(truncated);
    result["build_root_count"] = json!(discovery.build_roots.len());
    result["build_roots"] = json!(
        discovery
            .build_roots
            .iter()
            .take(MAX_ROOTS)
            .map(|(path, manifests)| json!({"path":path,"manifests":manifests}))
            .collect::<Vec<_>>()
    );
    result["manifest_count"] = json!(discovery.manifest_sha256.len());
    result["manifest_sha256"] = json!(bounded_hashes(&discovery.manifest_sha256));
    result["checker_config_count"] = json!(discovery.checker_config_sha256.len());
    result["checker_config_sha256"] = json!(bounded_hashes(&discovery.checker_config_sha256));
    result["checker_count"] = json!(discovery.checker_configurations.len());
    result["checker_configurations"] = json!(
        discovery
            .checker_configurations
            .iter()
            .take(MAX_ROWS)
            .collect::<Vec<_>>()
    );
    result["blocked_path_count"] = json!(discovery.blocked_paths.len());
    result["blocked_paths"] = json!(
        discovery
            .blocked_paths
            .iter()
            .take(MAX_REASONS)
            .collect::<Vec<_>>()
    );
    result["unknown_condition_count"] = json!(discovery.unknown_conditions.len());
    result["unknown_conditions"] = json!(
        discovery
            .unknown_conditions
            .iter()
            .take(MAX_REASONS)
            .collect::<Vec<_>>()
    );
    result
}
fn bounded_hashes(values: &BTreeMap<String, String>) -> BTreeMap<&str, &str> {
    values
        .iter()
        .take(MAX_ROWS)
        .map(|(path, digest)| (path.as_str(), digest.as_str()))
        .collect()
}
