//! 逐清单语言目标投影；声明、本机版本及有效模型不得互相替代。

use crate::discovery::DiscoveryReport;
use serde_json::{Value, json};

/// 保留每个构建根的直接声明及字节依据，不生成全局语言版本。
pub(crate) fn render(discovery: &DiscoveryReport) -> Vec<Value> {
    let mut targets = Vec::new();
    let models = discovery
        .maven_module_models
        .iter()
        .map(|(manifest, model)| (manifest, "java", &model.language_targets))
        .chain(
            discovery
                .cargo_module_models
                .iter()
                .map(|(manifest, model)| (manifest, "rust", &model.language_targets)),
        );
    for (manifest, language, values) in models {
        let Some(hash) = discovery.manifest_sha256.get(manifest) else {
            continue;
        };
        for (kind, value) in values {
            targets.push(json!({"language_id":language,"target_kind":kind,"value":value,"status":"declared_only","manifest_ref":manifest,"manifest_sha256":hash,"build_root":crate::module_graph::build_root(manifest)}));
        }
    }
    targets
}
