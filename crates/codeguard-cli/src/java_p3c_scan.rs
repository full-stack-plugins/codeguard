//! Java/P3C 的多文件局部观察；项目配置与原生结果分别记录，不推断完整质量通过。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use codeguard_adapters::{CheckerConfiguration, configured_p3c_rulesets};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::java_p3c_command::{Args, observe};
use crate::workspace_refresh::read_workspace_baseline;

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

/// 一轮原生 Java 检查的只读工具与执行上下文。
pub(crate) struct NativeContext<'a> {
    pub(crate) manifest_sha256: &'a BTreeMap<String, String>,
    pub(crate) maven_tool: Option<&'a Path>,
    pub(crate) java_home: Option<&'a Path>,
    pub(crate) maven_repo: Option<&'a Path>,
    pub(crate) repo_sha256: Option<&'a str>,
    pub(crate) deadline: Instant,
    pub(crate) cancelled: &'a AtomicBool,
}

/// 按源码所属的最近 Maven 构建根读取配置，并逐文件调用隔离原生探针。
pub(crate) fn observe_project(
    root: &Path,
    sources: &BTreeSet<String>,
    configurations: &[CheckerConfiguration],
    context: &NativeContext<'_>,
) -> Value {
    let mut files = Vec::with_capacity(sources.len());
    let mut observed_file_count = 0_usize;
    let mut finding_count = 0_usize;
    let mut findings = Vec::new();
    let mut occurrences = std::collections::BTreeMap::<String, u64>::new();
    let (workspace_binding, workspace_id) = match read_workspace_baseline(root) {
        Ok(Some(baseline)) => match baseline.workspace_id() {
            Some(id) => ("bound", json!(id)),
            None => ("legacy_unbound", Value::Null),
        },
        Ok(None) => ("uninitialized", Value::Null),
        Err(_) => ("invalid", Value::Null),
    };
    for relative in sources {
        let configuration = configurations
            .iter()
            .filter(|entry| {
                entry.checker_id == "java.maven.p3c"
                    && (entry.build_root == "."
                        || relative.starts_with(&format!("{}/", entry.build_root)))
            })
            .max_by_key(|entry| entry.build_root.len());
        let config_status = configuration.map_or("unknown", |entry| entry.configuration.as_str());
        let pom_ref = configuration.map(|entry| entry.configuration_ref.as_str());
        let pom_bytes = pom_ref.and_then(|reference| {
            read_bounded_regular_file(&root.join(reference), 4 * 1024 * 1024).ok()
        });
        let pom_sha = pom_bytes
            .as_ref()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
        let selected = pom_bytes.as_deref().and_then(configured_p3c_rulesets);
        let config_stable = pom_ref.is_some_and(|reference| {
            context.manifest_sha256.get(reference).map(String::as_str) == pom_sha.as_deref()
        });
        let (reason, observation) = if context.cancelled.load(Ordering::Relaxed)
            || codeguard_runtime::sigint_cancellation_requested()
        {
            ("request_cancelled", Value::Null)
        } else if Instant::now() >= context.deadline {
            ("request_deadline_exceeded", Value::Null)
        } else if config_status != "configured" {
            ("p3c_configuration_not_confirmed", Value::Null)
        } else if !config_stable {
            ("p3c_configuration_changed_before_scan", Value::Null)
        } else if selected.is_none() {
            ("p3c_ruleset_selection_unresolved", Value::Null)
        } else {
            let args = Args {
                source: root.join(relative),
                maven_tool: context.maven_tool.map(Path::to_path_buf),
                java_home: context.java_home.map(Path::to_path_buf),
                maven_repo: context.maven_repo.map(Path::to_path_buf),
                repo_sha256: context.repo_sha256.map(str::to_owned),
                selected_rulesets: selected,
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
                files.push(json!({
                    "path":relative, "build_root":configuration.map_or(".", |entry| entry.build_root.as_str()),
                    "configuration":config_status, "configuration_ref":pom_ref,
                    "configuration_sha256":pom_sha,
                    "reason":"p3c_configuration_changed_during_scan", "observation":null
                }));
                continue;
            }
            let status = report["local_status"].as_str().unwrap_or("incomplete");
            if matches!(
                status,
                "findings_observed_untrusted" | "clean_scope_unproven"
            ) {
                observed_file_count += 1;
            }
            let raw_count = report["findings"].as_array().map_or(0, Vec::len);
            finding_count += raw_count;
            let before = findings.len();
            if status == "findings_observed_untrusted" {
                let source = read_bounded_regular_file(&root.join(relative), 16 * 1024 * 1024);
                if let Ok(bytes) = source {
                    let source_sha = format!("{:x}", Sha256::digest(&bytes));
                    if report["source_sha256"] == source_sha {
                        for finding in report["findings"].as_array().into_iter().flatten() {
                            if let Some(projected) =
                                finding_record(relative, &bytes, finding, &mut occurrences)
                            {
                                findings.push(projected);
                            }
                        }
                    }
                }
            }
            if status == "findings_observed_untrusted" && findings.len() - before != raw_count {
                findings.truncate(before);
                observed_file_count -= 1;
                ("finding_projection_incomplete", report)
            } else {
                ("native_probe_returned", report)
            }
        };
        files.push(json!({
            "path":relative,
            "build_root":configuration.map_or(".", |entry| entry.build_root.as_str()),
            "configuration":config_status,
            "configuration_ref":configuration.map(|entry| entry.configuration_ref.as_str()),
            "configuration_sha256":pom_sha,
            "reason":reason,
            "observation":observation
        }));
    }
    json!({
        "schema_version":"0.2.0",
        "report_type":"java_p3c_project_observation",
        "operation":"lint", "command_status":"incomplete", "exit_code":3,
        "workspace_binding":workspace_binding, "workspace_id":workspace_id,
        "run_id":run_id(), "tool_approval":"unverified", "rulepack_approval":"unverified",
        "checker_id":"java.maven.p3c",
        "language":"java",
        "category":"lint",
        "source_file_count":sources.len(),
        "observed_file_count":observed_file_count,
        "finding_count":finding_count,
        "findings":findings,
        "local_observation_complete":observed_file_count == sources.len() && !sources.is_empty(),
        "coverage_proven":false,
        "authority":"local_unverified",
        "delivery_decision":"not_evaluated",
        "files":files
    })
}

/// 由本轮源码字节和原生诊断生成不依赖行号的稳定 finding 身份。
pub(crate) fn finding_record(
    relative: &str,
    bytes: &[u8],
    native: &Value,
    occurrences: &mut std::collections::BTreeMap<String, u64>,
) -> Option<Value> {
    let rule = native["rule_id"].as_str()?;
    let line = native["line"].as_u64().filter(|line| *line > 0)?;
    let anchor = bytes
        .split(|byte| *byte == b'\n')
        .nth(line.saturating_sub(1) as usize)?
        .trim_ascii();
    if anchor.is_empty() {
        return None;
    }
    let mut digest = Sha256::new();
    for part in [
        b"codeguard-p3c-finding-v1".as_slice(),
        relative.as_bytes(),
        rule.as_bytes(),
        anchor,
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    let base = format!("{:x}", digest.finalize());
    let ordinal = occurrences.entry(base.clone()).or_default();
    let mut final_digest = Sha256::new();
    final_digest.update(base.as_bytes());
    final_digest.update(ordinal.to_be_bytes());
    *ordinal += 1;
    let fingerprint = format!("{:x}", final_digest.finalize());
    Some(json!({
        "finding_id":format!("CG-{}", &fingerprint[..32]),
        "finding_fingerprint":fingerprint,
        "source_sha256":format!("{:x}", Sha256::digest(bytes)),
        "path":relative,
        "rule_id":rule,
        "line":line,
        "column":native["column"]
    }))
}

fn run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "java-p3c-{}-{nanos}-{}",
        std::process::id(),
        NEXT_RUN.fetch_add(1, Ordering::Relaxed)
    )
}
