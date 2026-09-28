//! 文档任务的同轮普通/强制告警复检；保持局部观察与正式关闭分离。
use crate::discovery::discover;
use crate::rust_comments_command::observe_for_verification;
use codeguard_adapters::legacy_registry;
use codeguard_runtime::{NativeObservation, SourceSnapshot, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// 同一截止时间下执行原生文档复检和抑制对照。
/// 参数为当前项目、任务简报、显式Cargo及预算；返回绑定输入的局部报告或前置错误。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
    timeout: u64,
    source: &str,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let registry = legacy_registry().map_err(|_| "language_registry_invalid")?;
    let discovery = discover(root, &registry, &NativeObservation);
    let sources = discovery
        .languages
        .get("rust")
        .ok_or("rust_source_scope_unavailable")?;
    if !discovery.observation_complete {
        return Err("source_discovery_incomplete");
    }
    let mut paths: Vec<PathBuf> = sources.source_files.iter().map(PathBuf::from).collect();
    paths.extend([PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")]);
    let snapshot =
        SourceSnapshot::capture(root, paths, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024).ok();
    let normal = observe_for_verification(
        root,
        tool,
        timeout,
        source,
        deadline,
        false,
        &std::sync::atomic::AtomicBool::new(false),
    );
    let forced = if brief["kind"] == "finding" && normal["local_scan_complete"] == true {
        Some(observe_for_verification(
            root,
            tool,
            timeout,
            source,
            deadline,
            true,
            &std::sync::atomic::AtomicBool::new(false),
        ))
    } else {
        None
    };
    let inputs:Vec<Value>=snapshot.as_ref().map(|s|s.files().iter().map(|(path,bytes)|json!({"path":path,"sha256":format!("{:x}",Sha256::digest(bytes))})).collect()).unwrap_or_default();
    let after = discover(root, &registry, &NativeObservation);
    let stable = snapshot
        .as_ref()
        .is_some_and(|s| s.verify_unchanged(root).unwrap_or(false))
        && after.observation_complete
        && after.languages.get("rust").map(|v| &v.source_files) == Some(&sources.source_files)
        && forced.as_ref().is_none_or(|f| {
            f["tool_sha256"] == normal["tool_sha256"]
                && f["manifest_sha256"] == normal["manifest_sha256"]
                && f["lock_sha256"] == normal["lock_sha256"]
        });
    Ok(
        json!({"schema_version":"0.1.0","report_type":"rustdoc_task_recheck","operation":"task_verify",
        "workspace_binding":normal["workspace_binding"],"workspace_id":normal["workspace_id"],"run_id":normal["run_id"],
        "authority":"local_unverified","delivery_decision":"not_evaluated","coverage_proven":false,
        "task_id":brief["task_id"],"task_path":if brief["kind"]=="finding" {brief["scope"].clone()} else {Value::Null},
        "task_rule":brief["native_rule_id"],"input_stable":stable,"input_identities":inputs,
        "normal_scan":normal,"forced_scan":forced}),
    )
}

/// 保存之前重新核对已捕获输入；返回当前字节是否仍匹配，不证明覆盖或批准。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    report["input_stable"] == true
        && report["input_identities"].as_array().is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row["path"].as_str().is_some_and(|path| {
                        !path.is_empty()
                            && !path.starts_with('/')
                            && !path.contains(['\\', ':'])
                            && path.split('/').all(|p| !matches!(p, "" | "." | ".."))
                            && read_bounded_regular_file(&root.join(path), 4 * 1024 * 1024)
                                .is_ok_and(|bytes| {
                                    row["sha256"] == format!("{:x}", Sha256::digest(bytes))
                                })
                    })
                })
        })
}

/// 将原生观察映射为局部任务结果；零发现或抑制不关闭任务。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    let normal = &report["normal_scan"];
    if report["report_type"] != "rustdoc_task_recheck"
        || report["task_id"] != brief["task_id"]
        || report["input_stable"] != true
        || normal["local_scan_complete"] != true
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
    let Some(items) = normal["findings"].as_array() else {
        return "incomplete";
    };
    if items.iter().any(|f| {
        f["finding_id"] == brief["task_id"]
            && f["path"] == brief["scope"]
            && f["rule_id"] == brief["native_rule_id"]
    }) {
        return "still_present";
    }
    if items
        .iter()
        .any(|f| f["path"] == brief["scope"] && f["rule_id"] == brief["native_rule_id"])
    {
        return "rule_coverage_requires_review";
    }
    let forced = &report["forced_scan"];
    if forced["local_scan_complete"] != true
        || forced["tool_sha256"] != normal["tool_sha256"]
        || forced["manifest_sha256"] != normal["manifest_sha256"]
        || forced["lock_sha256"] != normal["lock_sha256"]
    {
        return "incomplete";
    }
    if forced["findings"].as_array().is_some_and(|rows| {
        rows.iter()
            .any(|f| f["path"] == brief["scope"] && f["rule_id"] == brief["native_rule_id"])
    }) {
        "suppression_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

/// 核对复检封套与原生扫描的绑定；不核验批准来源。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let normal = &report["normal_scan"];
    report.as_object().is_some_and(|v| {
        v.len() == 16
            && [
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
                "forced_scan",
            ]
            .iter()
            .all(|key| v.contains_key(*key))
    }) && report["schema_version"] == "0.1.0"
        && report["report_type"] == "rustdoc_task_recheck"
        && report["operation"] == "task_verify"
        && report["authority"] == "local_unverified"
        && report["delivery_decision"] == "not_evaluated"
        && report["coverage_proven"] == false
        && report["workspace_binding"] == normal["workspace_binding"]
        && report["workspace_id"] == normal["workspace_id"]
        && report["run_id"] == normal["run_id"]
        && report["task_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("CG-"))
        && report["input_stable"].is_boolean()
        && valid_input_ownership(report)
        && normal["report_type"] == "rustdoc_local_observation"
        && normal["schema_version"] == "0.4.0"
        && normal["checker_id"] == "rust.cargo_rustdoc"
        && (report["forced_scan"].is_null() || {
            let forced = &report["forced_scan"];
            forced["report_type"] == "rustdoc_local_observation"
                && forced["schema_version"] == "0.4.0"
                && forced["checker_id"] == "rust.cargo_rustdoc"
                && forced["workspace_id"] == normal["workspace_id"]
                && forced["recheck_argv"]
                    == json!([
                        "cargo",
                        "rustdoc",
                        "--lib",
                        "--locked",
                        "--offline",
                        "--message-format=json",
                        "--",
                        "--force-warn",
                        "missing_docs",
                        "--force-warn",
                        "rustdoc::broken_intra_doc_links"
                    ])
        })
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
        if !row.as_object().is_some_and(|value| {
            value.len() == 2 && value.contains_key("path") && value.contains_key("sha256")
        }) {
            return false;
        }
        let Some(path) = row["path"].as_str().filter(|path| {
            !path.is_empty()
                && !path.starts_with('/')
                && !path.contains(['\\', ':'])
                && path
                    .split('/')
                    .all(|p| !matches!(p, "" | "." | "..") && !p.chars().any(char::is_control))
        }) else {
            return false;
        };
        let Some(sha) = row["sha256"].as_str().filter(|sha| {
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        }) else {
            return false;
        };
        if inputs.insert(path, sha).is_some() {
            return false;
        }
    }
    if report["input_stable"] != true {
        return true;
    }
    let normal = &report["normal_scan"];
    for (path, field) in [
        ("Cargo.toml", "manifest_sha256"),
        ("Cargo.lock", "lock_sha256"),
    ] {
        if normal[field]
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
    for scan in [normal, &report["forced_scan"]] {
        if scan.is_null() {
            continue;
        }
        if scan["local_scan_complete"] == true
            && scan["findings"].as_array().is_none_or(|rows| {
                rows.iter().any(|f| {
                    f["path"]
                        .as_str()
                        .is_none_or(|path| inputs.get(path).copied() != f["source_sha256"].as_str())
                        || f["target_path"]
                            .as_str()
                            .is_none_or(|path| !inputs.contains_key(path))
                })
            })
        {
            return false;
        }
        if scan["local_scan_complete"] == true
            && ["tool_sha256", "manifest_sha256", "lock_sha256"]
                .iter()
                .any(|field| scan[*field] != normal[*field])
        {
            return false;
        }
    }
    true
}
