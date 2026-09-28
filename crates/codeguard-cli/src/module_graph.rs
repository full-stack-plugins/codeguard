//! 保留结构、Maven 聚合与直接依赖声明的来源；未知模型不能用于缩小检查范围。

use crate::discovery::DiscoveryReport;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// 生成仅供上下文解释的模块图；完整性始终保留为未解析。
pub(crate) fn render(discovery: &DiscoveryReport) -> Value {
    let mut nodes: Vec<Value> = discovery
        .build_roots
        .keys()
        .map(|path| json!({"id":path,"kind":"build_root","status":"observed"}))
        .collect();
    if !discovery.build_roots.contains_key(".") {
        nodes.insert(
            0,
            json!({"id":".","kind":"project_root","status":"observed"}),
        );
    }
    let mut edges: Vec<Value> = discovery.build_roots.keys().filter(|path| path.as_str() != ".")
        .map(|path| json!({"from":".","to":path,"kind":"contains","status":"observed","basis":"manifest_location"})).collect();
    let mut unresolved = BTreeSet::from([
        "build_dependencies_not_resolved".to_owned(),
        "source_references_not_resolved".to_owned(),
    ]);
    let mut coordinates = BTreeMap::<(String, String, String), Vec<String>>::new();
    for (manifest, model) in &discovery.maven_module_models {
        if let Some(identity) = &model.coordinates {
            coordinates
                .entry(identity.clone())
                .or_default()
                .push(build_root(manifest));
        }
    }
    for manifests in discovery.build_roots.values() {
        for manifest in manifests
            .iter()
            .filter(|manifest| manifest.ends_with("pom.xml"))
        {
            if !discovery.maven_module_models.contains_key(manifest) {
                unresolved.insert(format!("maven_manifest_unresolved:{manifest}"));
            }
        }
    }
    for (manifest, model) in &discovery.maven_module_models {
        let from = build_root(manifest);
        for reason in &model.unresolved {
            unresolved.insert(format!("maven_{reason}:{manifest}"));
        }
        if model
            .coordinates
            .as_ref()
            .is_some_and(|identity| coordinates[identity].len() != 1)
        {
            unresolved.insert(format!("maven_project_coordinates_ambiguous:{manifest}"));
        }
        let Some(hash) = discovery.manifest_sha256.get(manifest) else {
            unresolved.insert(format!("maven_manifest_identity_missing:{manifest}"));
            continue;
        };
        let mut modules = BTreeSet::new();
        for module in &model.modules {
            let Some(to) = local_module_path(&from, module) else {
                unresolved.insert(format!("maven_module_path_unresolved:{manifest}"));
                continue;
            };
            let target_manifest = if to == "." {
                "pom.xml".into()
            } else {
                format!("{to}/pom.xml")
            };
            if to == from
                || !discovery.maven_module_models.contains_key(&target_manifest)
                || !modules.insert(to.clone())
            {
                unresolved.insert(format!("maven_module_target_unresolved:{manifest}"));
                continue;
            }
            edges.push(json!({"from":from,"to":to,"kind":"aggregation","status":"declared","basis":"maven_modules_declaration","manifest_ref":manifest,"manifest_sha256":hash,"condition":"unconditional_declaration"}));
        }
        let mut emitted = BTreeSet::new();
        for (group, artifact, version, scope) in &model.dependencies {
            let identity = (group.clone(), artifact.clone(), version.clone());
            let targets = coordinates.get(&identity);
            if model
                .coordinates
                .as_ref()
                .is_none_or(|id| coordinates[id].len() != 1)
                || targets.is_none_or(|targets| targets.len() != 1 || targets[0] == from)
            {
                unresolved.insert(format!("maven_dependency_target_unresolved:{manifest}"));
                continue;
            }
            let to = &targets.unwrap()[0];
            if !emitted.insert((to.clone(), scope.clone())) {
                unresolved.insert(format!("maven_dependency_duplicate:{manifest}"));
                continue;
            }
            edges.push(json!({"from":from,"to":to,"kind":"build_dependency","status":"declared","basis":"maven_dependency_declaration","manifest_ref":manifest,"manifest_sha256":hash,"scope":scope,"condition":"unconditional_declaration"}));
        }
    }
    crate::cargo_module_graph::append(discovery, &mut edges, &mut unresolved);
    json!({"schema_version":"0.3.0","document_type":"codeguard_module_graph","nodes":nodes,"edges":edges,"dependency_edges_complete":false,"unresolved":unresolved})
}

pub(crate) fn build_root(manifest: &str) -> String {
    manifest
        .rsplit_once('/')
        .map_or(".", |(parent, _)| parent)
        .into()
}
pub(crate) fn local_module_path(from: &str, module: &str) -> Option<String> {
    if module.starts_with('/') || module.contains(['\\', ':']) {
        return None;
    }
    let mut parts: Vec<&str> = if from == "." {
        Vec::new()
    } else {
        from.split('/').collect()
    };
    for part in module.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            value => parts.push(value),
        }
    }
    Some(if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    })
}
