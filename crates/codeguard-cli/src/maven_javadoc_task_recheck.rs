//! 以首次Maven多文件上下文复检Javadoc任务；不从报告执行工具或授予关闭。
use crate::{
    discovery::discover,
    java_javadoc_scan::{NativeContext, observe_project},
    maven_javadoc_workbench::prepare,
    tool_identity::hash_bundle_tree,
};
use codeguard_adapters::{legacy_registry, parse_unique_json};
use codeguard_runtime::{NativeObservation, SourceSnapshot, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// 根据固定首次证据确定构建根，显式当前工具必须与已有身份匹配。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    context: &NativeContext<'_>,
) -> Result<Value, &'static str> {
    let first = original(root, brief)?;
    let task_path = brief["scope"]
        .as_str()
        .ok_or("maven_javadoc_task_scope_invalid")?;
    let kind = brief["kind"]
        .as_str()
        .ok_or("maven_javadoc_task_kind_invalid")?;
    let build = if kind == "finding" {
        first["native"]["files"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r["path"] == task_path))
            .and_then(|r| r["build_root"].as_str())
            .ok_or("maven_javadoc_original_scope_invalid")?
    } else {
        task_path
    };
    let prefix = if build == "." {
        String::new()
    } else {
        format!("{build}/")
    };
    let config = format!("{prefix}pom.xml");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut report = json!({"schema_version":"0.1.0","report_type":"maven_javadoc_task_recheck","operation":"task_verify","run_id":format!("javadoc-maven-task-{}-{nanos}",std::process::id()),"workspace_binding":"bound","workspace_id":first["workspace_id"],"checker_id":"java.maven.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","task_id":brief["task_id"],"task_path":task_path,"task_rule":brief["native_rule_id"],"origin":brief["evidence_ref"],"scan":null,"input_bindings":[],"task_input_stable":false,"configuration_matches":false,"tool_identity_matches":false,"source_scope_matches":false,"reason":"maven_javadoc_recheck_incomplete"});
    let (Some(maven), Some(home), Some(repo), Some(repo_sha)) = (
        context.maven_tool,
        context.java_home,
        context.maven_repo,
        context.repo_sha256,
    ) else {
        report["reason"] = json!("prerequisites_missing");
        return Ok(report);
    };
    let registry = legacy_registry().map_err(|_| "language_registry_invalid")?;
    let discovery = discover(root, &registry, &NativeObservation);
    if !discovery.observation_complete {
        report["reason"] = json!("source_discovery_incomplete");
        return Ok(report);
    }
    let sources: BTreeSet<String> = discovery
        .languages
        .get("java")
        .map(|l| {
            l.source_files
                .iter()
                .filter(|path| {
                    let nearest = discovery
                        .checker_configurations
                        .iter()
                        .filter(|c| {
                            c.checker_id == "java.maven.javadoc"
                                && (c.build_root == "."
                                    || path.starts_with(&format!("{}/", c.build_root)))
                        })
                        .max_by_key(|c| c.build_root.len());
                    nearest.is_some_and(|c| c.build_root == build && c.configuration_ref == config)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if sources.is_empty() || (kind == "finding" && !sources.contains(task_path)) {
        report["reason"] = json!("maven_javadoc_source_not_selected");
        return Ok(report);
    }
    let mut paths: BTreeSet<PathBuf> = sources.iter().map(PathBuf::from).collect();
    paths.insert(PathBuf::from(&config));
    let snapshot =
        match SourceSnapshot::capture(root, paths, 2_001, 16 * 1024 * 1024, 128 * 1024 * 1024) {
            Ok(s) => s,
            Err(_) => {
                report["reason"] = json!("source_snapshot_unavailable");
                return Ok(report);
            }
        };
    let mut bindings: Vec<Value> = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p.to_string_lossy(),"sha256":digest(b),"location":"workspace"}))
        .collect();
    let mut tool_hashes = Vec::new();
    for path in [
        maven.to_owned(),
        home.join("bin/java"),
        home.join("release"),
    ] {
        let bytes = match read_bounded_regular_file(&path, 128 * 1024 * 1024) {
            Ok(b) => b,
            Err(_) => {
                report["reason"] = json!("native_tool_identity_unavailable");
                return Ok(report);
            }
        };
        let sha = digest(&bytes);
        tool_hashes.push(sha.clone());
        bindings.push(json!({"path":path,"sha256":sha,"location":"tool"}));
    }
    if hash_bundle_tree(repo).ok().as_deref() != Some(repo_sha) {
        report["reason"] = json!("dependency_closure_mismatch");
        return Ok(report);
    }
    bindings.push(json!({"path":repo,"sha256":repo_sha,"location":"bundle"}));
    report["input_bindings"] = json!(bindings);
    let old = first["native"]["maven_multifile_probes"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["build_root"] == build))
        .map(|r| &r["observation"]);
    let tools_match = old.is_none_or(|old| {
        [
            "maven_tool_sha256",
            "java_runtime_sha256",
            "dependency_closure_sha256",
        ]
        .iter()
        .zip([tool_hashes[0].as_str(), tool_hashes[1].as_str(), repo_sha])
        .all(|(key, sha)| old[*key].is_null() || old[*key] == sha)
    });
    report["tool_identity_matches"] = json!(tools_match);
    if !tools_match {
        report["reason"] = json!("original_tool_identity_mismatch");
        report["task_input_stable"] = json!(inputs_current(root, &report));
        return Ok(report);
    }
    let expected = first["inputs"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["path"] == config))
        .and_then(|r| r["sha256"].as_str());
    report["configuration_matches"] = json!(
        snapshot
            .files()
            .get(Path::new(&config))
            .map(|b| digest(b))
            .as_deref()
            == expected
    );
    let old_sources: BTreeSet<String> = first["native"]["files"]
        .as_array()
        .ok_or("maven_original_files_invalid")?
        .iter()
        .filter(|r| r["build_root"] == build)
        .filter_map(|r| r["path"].as_str().map(str::to_owned))
        .collect();
    report["source_scope_matches"] = json!(sources == old_sources);
    let native = observe_project(
        root,
        &sources,
        &discovery.checker_configurations,
        &NativeContext {
            manifest_sha256: &discovery.manifest_sha256,
            java_home: Some(home),
            maven_tool: Some(maven),
            maven_repo: Some(repo),
            repo_sha256: Some(repo_sha),
            deadline: context.deadline,
            cancelled: context.cancelled,
        },
    );
    match prepare(root, &json!({"native_observation":native}), Some(&snapshot)) {
        Ok(mut scan) => {
            scan["run_id"] = report["run_id"].clone();
            report["scan"] = scan;
            report["task_input_stable"] = json!(inputs_current(root, &report));
            report["reason"] = json!("maven_javadoc_local_policy_unverified");
        }
        Err(reason) => report["reason"] = json!(reason),
    }
    Ok(report)
}
/// 记录前及查询历史时复核本轮源码、工具字节与离线仓库树摘要。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    let Some(rows) = report["input_bindings"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 2_005)
    else {
        return false;
    };
    rows.iter().all(|r| {
        let Some(path) = r["path"].as_str() else {
            return false;
        };
        if r["location"] == "bundle" {
            Path::new(path).is_absolute()
                && hash_bundle_tree(Path::new(path)).ok().as_deref() == r["sha256"].as_str()
        } else {
            crate::javadoc_task_recheck::inputs_current(root, &json!({"input_bindings":[r]}))
        }
    })
}
/// 只分类本地原任务观察，配置或源集变化需要审查，零诊断不能关闭。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if !valid_shape(report)
        || report["task_id"] != brief["task_id"]
        || report["task_path"] != brief["scope"]
        || report["task_rule"] != brief["native_rule_id"]
        || report["origin"] != brief["evidence_ref"]
        || report["task_input_stable"] != true
        || report["tool_identity_matches"] != true
    {
        return "incomplete";
    }
    let Some(probes) = report["scan"]["native"]["maven_multifile_probes"]
        .as_array()
        .filter(|r| r.len() == 1)
    else {
        return "incomplete";
    };
    if !matches!(
        probes[0]["observation"]["native_status"].as_str(),
        Some("findings_observed_untrusted" | "clean_log_unverified")
    ) {
        return "incomplete";
    }
    if report["configuration_matches"] != true || report["source_scope_matches"] != true {
        return "rule_coverage_requires_review";
    }
    if brief["kind"] == "blocker" {
        if report["scan"]["blockers"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r["id"] == brief["task_id"]))
        {
            return "incomplete";
        }
        return "candidate_absent_unverified_policy";
    }
    if report["scan"]["findings"].as_array().is_some_and(|rows| {
        rows.iter()
            .any(|r| r["path"] == brief["scope"] && r["rule_id"] == brief["native_rule_id"])
    }) {
        "still_present"
    } else {
        "candidate_absent_unverified_policy"
    }
}
/// 验证复检容器的严格形状；详细源码与原生投影由同步导入器重核。
pub(crate) fn valid_shape(r: &Value) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "operation",
        "run_id",
        "workspace_binding",
        "workspace_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "task_id",
        "task_path",
        "task_rule",
        "origin",
        "scan",
        "input_bindings",
        "task_input_stable",
        "configuration_matches",
        "tool_identity_matches",
        "source_scope_matches",
        "reason",
    ];
    r.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && r["schema_version"] == "0.1.0"
        && r["report_type"] == "maven_javadoc_task_recheck"
        && r["operation"] == "task_verify"
        && r["workspace_binding"] == "bound"
        && r["checker_id"] == "java.maven.javadoc"
        && r["authority"] == "local_unverified"
        && r["coverage_proven"] == false
        && r["delivery_decision"] == "not_evaluated"
        && r["workspace_id"].is_string()
        && r["task_id"].is_string()
        && r["task_path"].is_string()
        && r["reason"].is_string()
        && [
            "task_input_stable",
            "configuration_matches",
            "tool_identity_matches",
            "source_scope_matches",
        ]
        .iter()
        .all(|k| r[*k].is_boolean())
        && r["run_id"].as_str().is_some_and(|s| {
            s.starts_with("javadoc-maven-task-")
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
        && r["origin"].as_object().is_some_and(|o| {
            o.len() == 2 && o.contains_key("first_run_id") && o.contains_key("first_report_sha256")
        })
        && r["input_bindings"].as_array().is_some_and(|rows| {
            rows.len() <= 2_005
                && rows.iter().all(|row| {
                    row.as_object().is_some_and(|o| {
                        o.len() == 3
                            && o.contains_key("path")
                            && o.contains_key("sha256")
                            && o.contains_key("location")
                    }) && row["path"].is_string()
                        && row["sha256"].as_str().is_some_and(|s| {
                            s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                        && matches!(
                            row["location"].as_str(),
                            Some("workspace" | "tool" | "bundle")
                        )
                })
        })
        && (r["scan"].is_null() || r["scan"].is_object())
}
fn original(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .filter(|s| {
            s.starts_with("javadoc-maven-")
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
        .ok_or("maven_original_reference_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "maven_original_unavailable")?;
    if brief["evidence_ref"]["first_report_sha256"] != digest(&bytes) {
        return Err("maven_original_changed");
    }
    let r = parse_unique_json(&bytes).map_err(|_| "maven_original_invalid")?;
    if r["report_type"] != "maven_javadoc_workbench_observation"
        || r["schema_version"] != "0.1.0"
        || r["checker_id"] != "java.maven.javadoc"
        || r["run_id"] != run
        || !r[if brief["kind"] == "finding" {
            "findings"
        } else {
            "blockers"
        }]
        .as_array()
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row[if brief["kind"] == "finding" {
                    "finding_id"
                } else {
                    "id"
                }] == brief["task_id"]
            })
        })
    {
        return Err("maven_original_identity_invalid");
    }
    let current =
        crate::workspace_refresh::read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    if current.as_ref().and_then(|b| b.workspace_id()) != r["workspace_id"].as_str() {
        return Err("maven_original_workspace_invalid");
    }
    Ok(r)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
