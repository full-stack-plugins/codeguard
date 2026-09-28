//! Rust 编译任务复检；原生零诊断仍须核验项目策略。

use crate::discovery::discover;
use crate::rust_build_command::observe_for_verification;
use codeguard_adapters::legacy_registry;
use codeguard_runtime::{NativeObservation, SourceSnapshot};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 在同一截止时间内复检任务，并绑定项目输入；返回局部观察，不关闭任务。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
    timeout: u64,
    source: &str,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let registry = legacy_registry().map_err(|_| "language_registry_invalid")?;
    let before = discover(root, &registry, &NativeObservation);
    if !before.observation_complete {
        return Err("source_discovery_incomplete");
    }
    let sources = before
        .languages
        .get("rust")
        .ok_or("rust_source_scope_unavailable")?;
    let mut paths: Vec<PathBuf> = sources.source_files.iter().map(PathBuf::from).collect();
    paths.extend([PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")]);
    let snapshot =
        SourceSnapshot::capture(root, paths, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024).ok();
    let scan = observe_for_verification(
        root,
        tool,
        timeout,
        source,
        deadline,
        &AtomicBool::new(false),
    );
    let after = discover(root, &registry, &NativeObservation);
    let stable = snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.verify_unchanged(root).unwrap_or(false))
        && after.observation_complete
        && after
            .languages
            .get("rust")
            .map(|language| &language.source_files)
            == Some(&sources.source_files);
    let inputs: Vec<Value> = snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot.files().iter().map(|(path, bytes)| {
        json!({"path":path,"sha256":format!("{:x}",Sha256::digest(bytes))})
    }).collect()
        })
        .unwrap_or_default();
    Ok(json!({
        "schema_version":"0.1.0","report_type":"rust_build_task_recheck","operation":"task_verify",
        "workspace_binding":scan["workspace_binding"],"workspace_id":scan["workspace_id"],"run_id":scan["run_id"],
        "authority":"local_unverified","delivery_decision":"not_evaluated","coverage_proven":false,
        "task_id":brief["task_id"],"task_path":if brief["kind"]=="finding" {brief["scope"].clone()} else {Value::Null},
        "task_rule":brief["native_rule_id"],"input_stable":stable,"input_identities":inputs,"normal_scan":scan
    }))
}

/// 保存前再次捕获同一组输入，拒绝路径别名与已变化的源码。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    if report["input_stable"] != true {
        return false;
    }
    let Some(rows) = report["input_identities"]
        .as_array()
        .filter(|rows| !rows.is_empty() && rows.len() <= 10_000)
    else {
        return false;
    };
    let mut seen = BTreeSet::new();
    let mut paths = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(path) = row["path"].as_str().filter(|path| safe_path(path)) else {
            return false;
        };
        if !seen.insert(path) || !valid_sha(row["sha256"].as_str()) {
            return false;
        }
        paths.push(PathBuf::from(path));
    }
    let Ok(registry) = legacy_registry() else {
        return false;
    };
    let discovery = discover(root, &registry, &NativeObservation);
    let Some(language) = discovery.languages.get("rust") else {
        return false;
    };
    if !discovery.observation_complete {
        return false;
    }
    let expected: BTreeSet<&str> = language
        .source_files
        .iter()
        .map(String::as_str)
        .chain(["Cargo.toml", "Cargo.lock"])
        .collect();
    if seen != expected {
        return false;
    }
    let Ok(snapshot) =
        SourceSnapshot::capture(root, paths, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024)
    else {
        return false;
    };
    rows.iter().all(|row| {
        snapshot
            .files()
            .get(Path::new(row["path"].as_str().unwrap()))
            .is_some_and(|bytes| row["sha256"] == format!("{:x}", Sha256::digest(bytes)))
    })
}

/// 把本轮原生结果映射成候选状态；源码修复和策略批准均不在此决定。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    let scan = &report["normal_scan"];
    if report["report_type"] != "rust_build_task_recheck"
        || report["task_id"] != brief["task_id"]
        || report["input_stable"] != true
        || scan["local_scan_complete"] != true
    {
        return if brief["kind"] == "blocker" {
            "still_blocked"
        } else {
            "incomplete"
        };
    }
    if brief["kind"] == "blocker" {
        return "environment_restored_unverified_policy";
    }
    if report["task_path"] != brief["scope"] || report["task_rule"] != brief["native_rule_id"] {
        return "incomplete";
    }
    let Some(items) = scan["findings"].as_array() else {
        return "incomplete";
    };
    if items.iter().any(|finding| {
        finding["finding_id"] == brief["task_id"]
            && finding["path"] == brief["scope"]
            && finding["rule_id"] == brief["native_rule_id"]
    }) {
        "still_present"
    } else if items.iter().any(|finding| {
        finding["path"] == brief["scope"] && finding["rule_id"] == brief["native_rule_id"]
    }) {
        "rule_coverage_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

/// 核对复检封套的身份与必填输入；原生扫描由同步解析器另行严格核验。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let scan = &report["normal_scan"];
    let Some(object) = report.as_object() else {
        return false;
    };
    let keys = [
        "schema_version",
        "report_type",
        "operation",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "authority",
        "delivery_decision",
        "coverage_proven",
        "task_id",
        "task_path",
        "task_rule",
        "input_stable",
        "input_identities",
        "normal_scan",
    ];
    object.len() == keys.len()
        && keys.iter().all(|key| object.contains_key(*key))
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "rust_build_task_recheck"
        && report["operation"] == "task_verify"
        && report["authority"] == "local_unverified"
        && report["delivery_decision"] == "not_evaluated"
        && report["coverage_proven"] == false
        && report["workspace_binding"] == scan["workspace_binding"]
        && report["workspace_id"] == scan["workspace_id"]
        && report["run_id"] == scan["run_id"]
        && report["task_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("CG-"))
        && report["input_stable"].is_boolean()
        && valid_input_ownership(report)
        && scan["report_type"] == "rust_build_local_observation"
        && scan["schema_version"] == "0.2.0"
        && scan["checker_id"] == "rust.cargo_check"
}

fn valid_input_ownership(report: &Value) -> bool {
    let Some(rows) = report["input_identities"]
        .as_array()
        .filter(|rows| rows.len() <= 10_000)
    else {
        return false;
    };
    let mut inputs = BTreeMap::new();
    for row in rows {
        if !row.as_object().is_some_and(|item| {
            item.len() == 2 && item.contains_key("path") && item.contains_key("sha256")
        }) {
            return false;
        }
        let Some(path) = row["path"].as_str().filter(|path| safe_path(path)) else {
            return false;
        };
        let Some(sha) = row["sha256"].as_str().filter(|sha| valid_sha(Some(sha))) else {
            return false;
        };
        if inputs.insert(path, sha).is_some() {
            return false;
        }
    }
    if report["input_stable"] != true {
        return true;
    }
    let scan = &report["normal_scan"];
    for (path, field) in [
        ("Cargo.toml", "manifest_sha256"),
        ("Cargo.lock", "lock_sha256"),
    ] {
        if scan[field]
            .as_str()
            .is_some_and(|sha| inputs.get(path).copied() != Some(sha))
        {
            return false;
        }
    }
    if report["task_path"]
        .as_str()
        .is_some_and(|path| !inputs.contains_key(path))
    {
        return false;
    }
    if scan["local_scan_complete"] == true
        && scan["findings"].as_array().is_none_or(|rows| {
            rows.iter().any(|finding| {
                finding["path"].as_str().is_none_or(|path| {
                    inputs.get(path).copied() != finding["source_sha256"].as_str()
                }) || finding["target_path"]
                    .as_str()
                    .is_none_or(|path| !inputs.contains_key(path))
            })
        })
    {
        return false;
    }
    true
}

fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !matches!(part, "" | "." | "..") && !part.chars().any(char::is_control))
}

fn valid_sha(value: Option<&str>) -> bool {
    value.is_some_and(|sha| {
        sha.len() == 64
            && sha
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}
