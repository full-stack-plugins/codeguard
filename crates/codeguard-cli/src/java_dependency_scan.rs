//! 按 Java 源码最近构建根调度 Maven 依赖图原生局部探针。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use codeguard_adapters::CheckerConfiguration;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::maven_dependency_probe::{Request, incomplete, observe};

/// 本轮发现的 POM 与原生工具输入。
pub(crate) struct NativeContext<'a> {
    pub manifest_sha256: &'a BTreeMap<String, String>,
    pub maven_tool: Option<&'a Path>,
    pub java_home: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

/// 依赖图归属已声明插件的构建根；无 Java 源文件的 POM 仍有依赖义务。
pub(crate) fn observe_project(
    root: &Path,
    sources: &BTreeSet<String>,
    configurations: &[CheckerConfiguration],
    context: &NativeContext<'_>,
) -> Value {
    let roots: BTreeSet<_> = configurations
        .iter()
        .filter(|entry| {
            entry.checker_id == "java.maven.dependency" && entry.configuration == "configured"
        })
        .map(|entry| entry.build_root.as_str())
        .collect();
    let mut probes = Vec::new();
    let mut observed = 0_usize;
    for build_root in roots {
        let entry = configurations.iter().find(|entry| {
            entry.build_root == build_root && entry.checker_id == "java.maven.dependency"
        });
        let Some(entry) = entry else {
            continue;
        };
        if entry.configuration != "configured" {
            continue;
        }
        let stable =
            read_bounded_regular_file(&root.join(&entry.configuration_ref), 4 * 1024 * 1024)
                .ok()
                .is_some_and(|bytes| {
                    context
                        .manifest_sha256
                        .get(&entry.configuration_ref)
                        .map(String::as_str)
                        == Some(format!("{:x}", Sha256::digest(bytes)).as_str())
                });
        let observation = if stable {
            observe(&Request {
                build_root: &root.join(build_root),
                expected_pom_sha256: context
                    .manifest_sha256
                    .get(&entry.configuration_ref)
                    .expect("已确认 POM 摘要"),
                maven_tool: context.maven_tool,
                java_home: context.java_home,
                maven_repo: context.maven_repo,
                repo_sha256: context.repo_sha256,
                deadline: context.deadline,
                cancelled: context.cancelled,
            })
        } else {
            incomplete("dependency_configuration_changed_before_scan")
        };
        if observation["native_status"] == "graph_observed_untrusted" {
            observed += 1;
        }
        probes.push(json!({
            "build_root":build_root,"configuration_ref":entry.configuration_ref,
            "observation":observation
        }));
    }
    json!({
        "schema_version":"0.2.0","report_type":"java_dependency_project_probe",
        "checker_id":"java.maven.dependency","language":"java","category":"dependencies",
        "source_file_count":sources.len(),"configured_build_root_count":probes.len(),
        "observed_graph_count":observed,
        "local_probe_complete":!probes.is_empty() && observed == probes.len(),
        "project_checker_attribution":"unverified","coverage_proven":false,
        "authority":"local_unverified","delivery_decision":"not_evaluated",
        "probes":probes
    })
}
