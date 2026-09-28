//! Ruff 复检配置的当前身份复核；只读发现不等于受批准的规则覆盖。

use std::path::Path;

use codeguard_adapters::legacy_registry;
use codeguard_runtime::NativeObservation;

use crate::discovery::discover;
use crate::python_lint_scan::select_checker;

/// 核对目标目前选中的配置路径及字节摘要；缺配置、优先级变化或发现不完整均返回假。
pub(crate) fn is_current(root: &Path, source: &str, config_ref: &str, config_sha: &str) -> bool {
    if config_sha.len() != 64
        || !config_sha
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return false;
    }
    let Ok(registry) = legacy_registry() else {
        return false;
    };
    let discovery = discover(root, &registry, &NativeObservation);
    if !discovery.observation_complete
        || !discovery
            .languages
            .get("python")
            .is_some_and(|python| python.source_files.contains(source))
    {
        return false;
    }
    select_checker(&discovery, source).is_some_and(|checker| {
        checker.configuration == "configured"
            && checker.configuration_ref == config_ref
            && discovery
                .checker_config_sha256
                .get(config_ref)
                .is_some_and(|current| current == config_sha)
    })
}
