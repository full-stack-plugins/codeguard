//! 已静态配置的 Java 项目使用 JDK Javadoc 作单文件诊断；不归因于 Maven 生效执行。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use codeguard_adapters::CheckerConfiguration;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::java_javadoc_command::{Args, observe};
use crate::maven_javadoc_probe::{
    Request as MavenRequest, incomplete_observation, observe as observe_maven,
};

pub(crate) struct NativeContext<'a> {
    pub manifest_sha256: &'a BTreeMap<String, String>,
    pub java_home: Option<&'a Path>,
    pub maven_tool: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

/// 对已静态确认 Javadoc 配置的主源码运行局部 JDK 探针。
pub(crate) fn observe_project(
    root: &Path,
    sources: &BTreeSet<String>,
    configurations: &[CheckerConfiguration],
    context: &NativeContext<'_>,
) -> Value {
    let mut files = Vec::with_capacity(sources.len());
    let mut observed_file_count = 0_usize;
    for relative in sources {
        let configuration = configurations
            .iter()
            .filter(|entry| {
                entry.checker_id == "java.maven.javadoc"
                    && (entry.build_root == "."
                        || relative.starts_with(&format!("{}/", entry.build_root)))
            })
            .max_by_key(|entry| entry.build_root.len());
        let config_status = configuration.map_or("unknown", |entry| entry.configuration.as_str());
        let pom_ref = configuration.map(|entry| entry.configuration_ref.as_str());
        let pom_sha = pom_ref.and_then(|reference| {
            read_bounded_regular_file(&root.join(reference), 4 * 1024 * 1024)
                .ok()
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        });
        let config_stable = pom_ref.is_some_and(|reference| {
            context.manifest_sha256.get(reference).map(String::as_str) == pom_sha.as_deref()
        });
        let source_in_main =
            relative.starts_with("src/main/java/") || relative.contains("/src/main/java/");
        let (reason, observation) = if context.cancelled.load(Ordering::Relaxed)
            || codeguard_runtime::sigint_cancellation_requested()
        {
            ("request_cancelled", Value::Null)
        } else if Instant::now() >= context.deadline {
            ("request_deadline_exceeded", Value::Null)
        } else if config_status != "configured" {
            ("javadoc_configuration_not_confirmed", Value::Null)
        } else if !config_stable {
            ("javadoc_configuration_changed_before_scan", Value::Null)
        } else if !source_in_main {
            ("javadoc_source_scope_unverified", Value::Null)
        } else if context.maven_tool.is_some() {
            ("maven_multifile_probe_selected", Value::Null)
        } else {
            let args = Args {
                source: root.join(relative),
                java_home: context.java_home.map(Path::to_path_buf),
                json: true,
            };
            let report = observe(&args, context.deadline, context.cancelled);
            let pom_still_stable = pom_ref.is_some_and(|reference| {
                read_bounded_regular_file(&root.join(reference), 4 * 1024 * 1024)
                    .ok()
                    .is_some_and(|bytes| {
                        format!("{:x}", Sha256::digest(bytes)) == pom_sha.as_deref().unwrap_or("")
                    })
            });
            if !pom_still_stable {
                ("javadoc_configuration_changed_during_scan", Value::Null)
            } else {
                if matches!(
                    report["local_status"].as_str(),
                    Some("findings_observed_untrusted" | "clean_scope_unproven")
                ) {
                    observed_file_count += 1;
                }
                ("native_probe_returned", report)
            }
        };
        files.push(json!({
            "path":relative,
            "build_root":configuration.map_or(".", |entry| entry.build_root.as_str()),
            "configuration":config_status,
            "configuration_ref":pom_ref,
            "configuration_sha256":pom_sha,
            "reason":reason,
            "observation":observation
        }));
    }
    let mut maven_probes = Vec::new();
    if context.maven_tool.is_some() {
        for configuration in configurations.iter().filter(|entry| {
            entry.checker_id == "java.maven.javadoc" && entry.configuration == "configured"
        }) {
            let prefix = if configuration.build_root == "." {
                String::new()
            } else {
                format!("{}/", configuration.build_root)
            };
            let selected: BTreeSet<String> = sources
                .iter()
                .filter_map(|source| source.strip_prefix(&prefix))
                .filter(|source| source.starts_with("src/main/java/"))
                .map(str::to_owned)
                .collect();
            if selected.is_empty() {
                continue;
            }
            let stable = read_bounded_regular_file(
                &root.join(&configuration.configuration_ref),
                4 * 1024 * 1024,
            )
            .ok()
            .is_some_and(|bytes| {
                context
                    .manifest_sha256
                    .get(&configuration.configuration_ref)
                    .map(String::as_str)
                    == Some(format!("{:x}", Sha256::digest(bytes)).as_str())
            });
            let observation = if stable {
                observe_maven(&MavenRequest {
                    build_root: &root.join(&configuration.build_root),
                    sources: &selected,
                    expected_pom_sha256: context
                        .manifest_sha256
                        .get(&configuration.configuration_ref)
                        .expect("已确认 POM 摘要稳定"),
                    maven_tool: context.maven_tool,
                    java_home: context.java_home,
                    maven_repo: context.maven_repo,
                    repo_sha256: context.repo_sha256,
                    deadline: context.deadline,
                    cancelled: context.cancelled,
                })
            } else {
                incomplete_observation("javadoc_configuration_changed_before_scan", selected.len())
            };
            maven_probes.push(json!({
                "build_root":configuration.build_root,
                "configuration_ref":configuration.configuration_ref,
                "observation":observation
            }));
        }
    }
    if context.maven_tool.is_some() {
        observed_file_count = maven_probes
            .iter()
            .filter(|probe| {
                matches!(
                    probe["observation"]["native_status"].as_str(),
                    Some("findings_observed_untrusted" | "clean_log_unverified")
                )
            })
            .filter_map(|probe| probe["observation"]["observed_source_count"].as_u64())
            .map(|count| count as usize)
            .sum();
    }
    json!({
        "schema_version":"0.3.0",
        "probe_mode":if context.maven_tool.is_some(){"maven_multifile"}else{"jdk_single_file"},
        "report_type":"java_javadoc_project_probe",
        "checker_id":if context.maven_tool.is_some(){"java.maven.javadoc"}else{"java.jdk.javadoc"},
        "project_checker_attribution":"unverified",
        "language":"java",
        "category":"comments",
        "source_file_count":sources.len(),
        "observed_file_count":observed_file_count,
        "local_probe_complete":observed_file_count == sources.len() && !sources.is_empty(),
        "coverage_proven":false,
        "authority":"local_unverified",
        "delivery_decision":"not_evaluated",
        "files":files,
        "maven_multifile_probes":maven_probes
    })
}
