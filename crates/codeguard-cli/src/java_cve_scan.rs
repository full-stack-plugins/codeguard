//! 已声明 OWASP 插件的 Maven 构建根调度；漏洞库不可信时保留原生观察与未完成状态。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use codeguard_adapters::CheckerConfiguration;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::owasp_maven_probe::{Request, incomplete, observe};
use crate::workspace_refresh::read_workspace_baseline;

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

/// 本轮扫描环境与发现的原始 POM 身份。
pub(crate) struct NativeContext<'a> {
    pub manifest_sha256: &'a BTreeMap<String, String>,
    pub maven_tool: Option<&'a Path>,
    pub java_home: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub data_dir: Option<&'a Path>,
    pub data_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

/// 按每个直接配置的构建根尝试原生检查；无 Java 源文件也保留依赖义务。
pub(crate) fn observe_project(
    root: &Path,
    configurations: &[CheckerConfiguration],
    context: &NativeContext<'_>,
) -> Value {
    let (workspace_binding, workspace_id) = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => ("bound", json!(id)),
            None => ("legacy_unbound", Value::Null),
        },
        Ok(None) => ("uninitialized", Value::Null),
        Err(_) => ("invalid", Value::Null),
    };
    let mut probes = Vec::new();
    let mut observed = 0usize;
    for entry in configurations.iter().filter(|entry| {
        entry.checker_id == "java.maven.dependency_check" && entry.configuration == "configured"
    }) {
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
                build_root: &root.join(&entry.build_root),
                expected_pom_sha256: context
                    .manifest_sha256
                    .get(&entry.configuration_ref)
                    .expect("已确认 POM 摘要"),
                maven_tool: context.maven_tool,
                java_home: context.java_home,
                maven_repo: context.maven_repo,
                repo_sha256: context.repo_sha256,
                data_dir: context.data_dir,
                data_sha256: context.data_sha256,
                deadline: context.deadline,
                cancelled: context.cancelled,
            })
        } else {
            incomplete("cve_configuration_changed_before_scan")
        };
        if observation["native_status"] != "incomplete" {
            observed += 1;
        }
        probes.push(json!({
            "build_root":entry.build_root,"configuration_ref":entry.configuration_ref,
            "observation":observation
        }));
    }
    json!({
        "schema_version":"0.3.0","report_type":"java_cve_project_probe",
        "operation":"check","command_status":"incomplete","exit_code":3,
        "workspace_binding":workspace_binding,"workspace_id":workspace_id,
        "run_id":run_id(),"tool_approval":"unverified","rulepack_approval":"unverified",
        "checker_id":"java.maven.dependency_check","language":"java","category":"cve",
        "configured_build_root_count":probes.len(),"observed_report_count":observed,
        "local_probe_complete":!probes.is_empty() && observed == probes.len(),
        "database_freshness":"unverified","coverage_proven":false,
        "authority":"local_unverified","delivery_decision":"not_evaluated",
        "probes":probes
    })
}

fn run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "java-cve-{}-{nanos}-{}",
        std::process::id(),
        NEXT_RUN.fetch_add(1, Ordering::Relaxed)
    )
}
