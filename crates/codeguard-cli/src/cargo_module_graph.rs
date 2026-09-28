//! Cargo 静态声明到模块图的投影；不把 path 声明当有效 Cargo 模型。

use crate::discovery::DiscoveryReport;
use crate::module_graph::{build_root, local_module_path};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) fn append(
    discovery: &DiscoveryReport,
    edges: &mut Vec<Value>,
    unresolved: &mut BTreeSet<String>,
) {
    for manifests in discovery.build_roots.values() {
        for manifest in manifests
            .iter()
            .filter(|manifest| manifest.ends_with("Cargo.toml"))
        {
            if !discovery.cargo_module_models.contains_key(manifest) {
                unresolved.insert(format!("cargo_manifest_unresolved:{manifest}"));
            }
        }
    }
    for (manifest, model) in &discovery.cargo_module_models {
        let from = build_root(manifest);
        for reason in &model.unresolved {
            unresolved.insert(format!("cargo_{reason}:{manifest}"));
        }
        let Some(hash) = discovery.manifest_sha256.get(manifest) else {
            unresolved.insert(format!("cargo_manifest_identity_missing:{manifest}"));
            continue;
        };
        let mut members = BTreeSet::new();
        for member in &model.members {
            let Some(to) = local_module_path(&from, member) else {
                unresolved.insert(format!("cargo_member_path_unresolved:{manifest}"));
                continue;
            };
            let target = manifest_at(&to);
            if to == from
                || discovery
                    .cargo_module_models
                    .get(&target)
                    .is_none_or(|model| model.package_name.is_none())
                || !members.insert(to.clone())
            {
                unresolved.insert(format!("cargo_member_target_unresolved:{manifest}"));
                continue;
            }
            edges.push(json!({"from":from,"to":to,"kind":"aggregation","status":"declared","basis":"cargo_members_declaration","manifest_ref":manifest,"manifest_sha256":hash,"condition":"unconditional_declaration"}));
        }
        let mut emitted = BTreeSet::new();
        for (package, path, scope) in &model.path_dependencies {
            let Some(to) = local_module_path(&from, path) else {
                unresolved.insert(format!("cargo_dependency_path_unresolved:{manifest}"));
                continue;
            };
            let target = discovery.cargo_module_models.get(&manifest_at(&to));
            if model.package_name.is_none()
                || to == from
                || target.is_none_or(|model| model.package_name.as_ref() != Some(package))
            {
                unresolved.insert(format!("cargo_dependency_target_unresolved:{manifest}"));
                continue;
            }
            if !emitted.insert((to.clone(), scope.clone())) {
                unresolved.insert(format!("cargo_dependency_duplicate:{manifest}"));
                continue;
            }
            edges.push(json!({"from":from,"to":to,"kind":"build_dependency","status":"declared","basis":"cargo_path_dependency_declaration","manifest_ref":manifest,"manifest_sha256":hash,"scope":scope,"condition":"unconditional_declaration"}));
        }
    }
}
fn manifest_at(root: &str) -> String {
    if root == "." {
        "Cargo.toml".into()
    } else {
        format!("{root}/Cargo.toml")
    }
}
