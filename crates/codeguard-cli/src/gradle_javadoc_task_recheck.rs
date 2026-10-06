//! Gradle文档任务的原始上下文复检；不执行报告命令，不授予可信关闭。
use crate::{
    gradle_javadoc_workbench::prepare, gradle_model_probe::Request, tool_identity::hash_bundle_tree,
};
use codeguard_runtime::{SourceSnapshot, read_bounded_regular_file};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// 复用原选定范围运行显式Gradle/JDK；参数为工作区、任务指引、工具路径、截止时间及取消状态。
/// 返回绑定的局部复检容器；Java字节允许修复，其余输入和已知工具身份不允许悄然变化。
pub fn run(
    root: &Path,
    brief: &Value,
    bundle: Option<&Path>,
    java: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    let first = original(root, brief)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut report = json!({"schema_version":"0.1.0","report_type":"gradle_javadoc_task_recheck","operation":"task_verify","run_id":format!("javadoc-gradle-task-{}-{nanos}",std::process::id()),"workspace_binding":"bound","workspace_id":first["workspace_id"],"checker_id":"java.gradle.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","task_id":brief["task_id"],"task_path":brief["scope"],"task_rule":brief["native_rule_id"],"origin":brief["evidence_ref"],"scan":null,"input_bindings":[],"task_input_stable":false,"configuration_matches":false,"tool_identity_matches":false,"original_tool_identity_complete":false,"source_scope_matches":false,"reason":"gradle_javadoc_recheck_incomplete"});
    let rows = first["inputs"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 128)
        .ok_or("gradle_original_inputs_invalid")?;
    let paths = rows
        .iter()
        .map(|r| {
            r["path"]
                .as_str()
                .map(PathBuf::from)
                .ok_or("gradle_original_input_invalid")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let snapshot =
        match SourceSnapshot::capture(root, paths.clone(), 128, 1024 * 1024, 16 * 1024 * 1024) {
            Ok(s) => s,
            Err(_) => {
                report["reason"] = json!("source_snapshot_unavailable");
                return Ok(report);
            }
        };
    let mut bindings = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":digest(b),"location":"workspace"}))
        .collect::<Vec<_>>();
    report["input_bindings"] = json!(bindings);
    report["source_scope_matches"] = json!(paths.len() == rows.len());
    let configuration_matches = rows
        .iter()
        .filter(|r| !r["path"].as_str().is_some_and(|p| p.ends_with(".java")))
        .all(|r| {
            snapshot
                .files()
                .get(Path::new(r["path"].as_str().unwrap()))
                .is_some_and(|b| r["sha256"] == digest(b))
        });
    report["configuration_matches"] = json!(configuration_matches);
    if !configuration_matches {
        report["task_input_stable"] = json!(inputs_current(root, &report));
        report["reason"] = json!("original_configuration_changed");
        return Ok(report);
    }
    let script = digest(
        format!(
            "{}\n{}",
            include_str!("../resources/gradle_checker_model.init.gradle"),
            include_str!("../resources/gradle_javadoc.init.gradle")
        )
        .as_bytes(),
    );
    if first["native"]["init_scripts_sha256"] != script {
        report["reason"] = json!("original_init_scripts_changed");
        return Ok(report);
    }
    let (Some(bundle), Some(java)) = (bundle, java) else {
        report["reason"] = json!("prerequisites_missing");
        let inputs = json!(
            snapshot
                .files()
                .iter()
                .map(|(p, b)| json!({"path":p,"sha256":digest(b)}))
                .collect::<Vec<_>>()
        );
        if let Ok(mut scan) = prepare(
            root,
            &inputs,
            &crate::gradle_javadoc_probe::missing_prerequisites(),
        ) {
            scan["run_id"] = report["run_id"].clone();
            report["scan"] = scan;
            report["task_input_stable"] = json!(
                snapshot.verify_source_unchanged().ok() == Some(true)
                    && inputs_current(root, &report)
            );
        }
        return Ok(report);
    };
    if !bundle.is_absolute() || !java.is_absolute() {
        report["reason"] = json!("native_tool_path_invalid");
        return Ok(report);
    }
    let bundle_sha = match hash_bundle_tree(bundle) {
        Ok(s) => s,
        Err(_) => {
            report["reason"] = json!("native_tool_identity_unavailable");
            return Ok(report);
        }
    };
    bindings.push(json!({"path":bundle,"sha256":bundle_sha,"location":"bundle"}));
    let mut tool_shas = vec![bundle_sha];
    for path in [java.join("bin/java"), java.join("release")] {
        let bytes = match read_bounded_regular_file(&path, 128 * 1024 * 1024) {
            Ok(b) => b,
            Err(_) => {
                report["reason"] = json!("native_tool_identity_unavailable");
                return Ok(report);
            }
        };
        let sha = digest(&bytes);
        tool_shas.push(sha.clone());
        bindings.push(json!({"path":path,"sha256":sha,"location":"tool"}));
    }
    report["input_bindings"] = json!(bindings);
    let keys = [
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
    ];
    report["original_tool_identity_complete"] =
        json!(keys.iter().all(|k| first["native"][*k].is_string()));
    let tools_match = keys
        .iter()
        .zip(&tool_shas)
        .all(|(k, s)| first["native"][*k].is_null() || first["native"][*k] == *s);
    report["tool_identity_matches"] = json!(tools_match);
    if !tools_match {
        report["task_input_stable"] = json!(inputs_current(root, &report));
        report["reason"] = json!("original_tool_identity_mismatch");
        return Ok(report);
    }
    let native = crate::gradle_javadoc_probe::observe(
        &Request {
            project_root: root.to_owned(),
            project_files: paths,
            gradle_bundle: bundle.to_owned(),
            java_home: java.to_owned(),
            deadline,
        },
        cancelled,
    );
    let inputs = json!(
        snapshot
            .files()
            .iter()
            .map(|(p, b)| json!({"path":p,"sha256":digest(b)}))
            .collect::<Vec<_>>()
    );
    match prepare(root, &inputs, &native) {
        Ok(mut scan) => {
            scan["run_id"] = report["run_id"].clone();
            report["scan"] = scan;
            report["task_input_stable"] = json!(
                snapshot.verify_source_unchanged().ok() == Some(true)
                    && inputs_current(root, &report)
            );
            report["reason"] = json!("gradle_javadoc_local_policy_unverified");
        }
        Err(reason) => report["reason"] = json!(reason),
    }
    Ok(report)
}

/// 复核当前源码和工具绑定；参数为工作区及复检容器，返回是否仍与记录一致。
pub fn inputs_current(root: &Path, report: &Value) -> bool {
    let Some(rows) = report["input_bindings"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 131)
    else {
        return false;
    };
    let mut workspace = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let Some(path) = row["path"].as_str() else {
            return false;
        };
        let Some(sha) = row["sha256"].as_str().filter(|s| valid_sha(s)) else {
            return false;
        };
        if !seen.insert((row["location"].as_str(), path)) {
            return false;
        }
        if row["location"] == "workspace" {
            workspace.insert(PathBuf::from(path));
        } else if row["location"] == "bundle" {
            if !Path::new(path).is_absolute()
                || hash_bundle_tree(Path::new(path)).ok().as_deref() != Some(sha)
            {
                return false;
            }
        } else if row["location"] == "tool" {
            if !Path::new(path).is_absolute()
                || read_bounded_regular_file(Path::new(path), 128 * 1024 * 1024)
                    .ok()
                    .is_none_or(|b| digest(&b) != sha)
            {
                return false;
            }
        } else {
            return false;
        }
    }
    let Ok(snapshot) = SourceSnapshot::capture(root, workspace, 128, 1024 * 1024, 16 * 1024 * 1024)
    else {
        return false;
    };
    rows.iter()
        .filter(|r| r["location"] == "workspace")
        .all(|r| {
            snapshot
                .files()
                .get(Path::new(r["path"].as_str().unwrap()))
                .is_some_and(|b| r["sha256"] == digest(b))
        })
        && snapshot.verify_source_unchanged().ok() == Some(true)
}

/// 分类原任务的局部观察；参数为指引和复检容器，返回观察类型，不授予关闭。
pub fn classify(brief: &Value, report: &Value) -> &'static str {
    if !valid_shape(report)
        || report["task_id"] != brief["task_id"]
        || report["task_path"] != brief["scope"]
        || report["task_rule"] != brief["native_rule_id"]
        || report["origin"] != brief["evidence_ref"]
        || report["task_input_stable"] != true
    {
        return "incomplete";
    }
    if report["reason"] == "original_configuration_changed" {
        return "rule_coverage_requires_review";
    }
    if report["tool_identity_matches"] != true
        || report["source_scope_matches"] != true
        || !matches!(
            report["scan"]["native"]["native_status"].as_str(),
            Some("findings_observed_unverified" | "empty_output_unverified")
        )
    {
        return "incomplete";
    }
    if report["configuration_matches"] != true || report["original_tool_identity_complete"] != true
    {
        return "rule_coverage_requires_review";
    }
    if brief["kind"] == "blocker" {
        return "rule_coverage_requires_review";
    }
    let findings = report["scan"]["findings"]
        .as_array()
        .expect("完整扫描形状已验证");
    let same_rule =
        |r: &Value| r["path"] == brief["scope"] && r["rule_id"] == brief["native_rule_id"];
    if findings
        .iter()
        .any(|r| same_rule(r) && r["finding_id"] == brief["task_id"])
    {
        "still_present"
    } else if findings.iter().any(same_rule) {
        // 同文件同规则的另一个行锚点不是原问题的确认，也不能作为消失/关闭证明。
        "rule_coverage_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

/// 验证容器封闭字段和身份；输入/原生投影由同步导入器重核。
pub fn valid_shape(r: &Value) -> bool {
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
        "original_tool_identity_complete",
        "source_scope_matches",
        "reason",
    ];
    r.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && r["schema_version"] == "0.1.0"
        && r["report_type"] == "gradle_javadoc_task_recheck"
        && r["operation"] == "task_verify"
        && r["workspace_binding"] == "bound"
        && r["checker_id"] == "java.gradle.javadoc"
        && r["authority"] == "local_unverified"
        && r["coverage_proven"] == false
        && r["delivery_decision"] == "not_evaluated"
        && r["workspace_id"].is_string()
        && r["task_id"]
            .as_str()
            .is_some_and(crate::next_command::safe_id)
        && r["task_path"].is_string()
        && (r["task_rule"].is_null() || r["task_rule"].is_string())
        && r["reason"].is_string()
        && [
            "task_input_stable",
            "configuration_matches",
            "tool_identity_matches",
            "original_tool_identity_complete",
            "source_scope_matches",
        ]
        .iter()
        .all(|k| r[*k].is_boolean())
        && r["run_id"].as_str().is_some_and(|s| {
            s.len() <= 120
                && s.starts_with("javadoc-gradle-task-")
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && r["origin"].as_object().is_some_and(|o| {
            o.len() == 2 && o.contains_key("first_run_id") && o.contains_key("first_report_sha256")
        })
        && r["origin"]["first_report_sha256"]
            .as_str()
            .is_some_and(valid_sha)
        && r["origin"]["first_run_id"].as_str().is_some_and(|s| {
            s.len() <= 120
                && s.starts_with("javadoc-gradle-")
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && r["input_bindings"].as_array().is_some_and(|rows| {
            rows.len() <= 131
                && rows.iter().all(|row| {
                    row.as_object().is_some_and(|o| {
                        o.len() == 3
                            && o.contains_key("path")
                            && o.contains_key("sha256")
                            && o.contains_key("location")
                    }) && row["path"].is_string()
                        && row["sha256"].as_str().is_some_and(valid_sha)
                        && matches!(
                            row["location"].as_str(),
                            Some("workspace" | "tool" | "bundle")
                        )
                })
        })
        && (r["scan"].is_null() || valid_scan(r))
}
// 局部扫描仍须具有完整、封闭的工作台和原生协议；缺字段不能被解释为零问题。
fn valid_scan(r: &Value) -> bool {
    let v = &r["scan"];
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "inputs",
        "native",
        "findings",
        "blockers",
    ];
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && v["schema_version"] == "0.1.0"
        && v["report_type"] == "gradle_javadoc_workbench_observation"
        && v["workspace_binding"] == "bound"
        && v["workspace_id"] == r["workspace_id"]
        && v["run_id"] == r["run_id"]
        && v["checker_id"] == r["checker_id"]
        && v["authority"] == "local_unverified"
        && v["coverage_proven"] == false
        && v["delivery_decision"] == "not_evaluated"
        && v["inputs"].as_array().is_some_and(|rows| {
            !rows.is_empty()
                && rows.len() <= 128
                && rows.iter().all(|row| {
                    row.as_object().is_some_and(|o| {
                        o.len() == 2 && o.contains_key("path") && o.contains_key("sha256")
                    }) && row["path"].is_string()
                        && row["sha256"].as_str().is_some_and(valid_sha)
                })
        })
        && v["findings"].is_array()
        && v["blockers"].is_array()
        && crate::gradle_javadoc_workbench::valid_native(&v["native"]).is_ok()
        && projection_agrees(v)
}

// 复检分类不能相信被删减的投影数组；原生定位归并后必须与投影完全一致。
fn projection_agrees(scan: &Value) -> bool {
    let Some(native) = scan["native"]["findings"]
        .as_array()
        .filter(|r| r.len() <= 10000)
    else {
        return false;
    };
    let Some(projected) = scan["findings"].as_array().filter(|r| r.len() <= 10000) else {
        return false;
    };
    let locations = |rows: &[Value]| {
        rows.iter()
            .map(|r| {
                Some((
                    r["path"].as_str()?.to_owned(),
                    r["rule_id"].as_str()?.to_owned(),
                    r["line"].as_u64()?,
                    r["column"].as_u64()?,
                ))
            })
            .collect::<Option<BTreeSet<_>>>()
    };
    match (locations(native), locations(projected)) {
        (Some(a), Some(b)) => a == b && b.len() == projected.len(),
        _ => false,
    }
}

/// 核对首次报告、消费收据和任务原生身份，返回原选定输入；不要求历史源码仍相同。
pub(crate) fn original(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    if brief["checker_id"] != "java.gradle.javadoc"
        || !matches!(brief["kind"].as_str(), Some("finding" | "blocker"))
    {
        return Err("gradle_original_identity_invalid");
    }
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .filter(|s| {
            s.len() <= 120
                && s.starts_with("javadoc-gradle-")
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .ok_or("gradle_original_reference_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "gradle_original_unavailable")?;
    let sha = digest(&bytes);
    if brief["evidence_ref"]["first_report_sha256"] != sha {
        return Err("gradle_original_changed");
    }
    let mut report =
        codeguard_adapters::parse_unique_json(&bytes).map_err(|_| "gradle_original_invalid")?;
    if report["report_type"] == "gradle_javadoc_task_recheck" {
        if !valid_shape(&report) {
            return Err("gradle_original_invalid");
        }
        report = report["scan"].clone();
    }
    let baseline =
        crate::workspace_refresh::read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let workspace = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let receipt = read_bounded_regular_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )
    .map_err(|_| "gradle_original_receipt_unavailable")?;
    let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":workspace,"run_id":run,"report_sha256":sha})).map_err(|_|"gradle_original_encoding_invalid")?;
    if receipt != expected
        || report["workspace_id"] != workspace
        || report["run_id"] != run
        || report["report_type"] != "gradle_javadoc_workbench_observation"
        || report["checker_id"] != "java.gradle.javadoc"
        || !report[if brief["kind"] == "finding" {
            "findings"
        } else {
            "blockers"
        }]
        .as_array()
        .is_some_and(|rows| {
            rows.iter().any(|r| {
                r[if brief["kind"] == "finding" {
                    "finding_id"
                } else {
                    "id"
                }] == brief["task_id"]
                    && r[if brief["kind"] == "finding" {
                        "path"
                    } else {
                        "scope"
                    }] == brief["scope"]
                    && (brief["kind"] != "finding" || r["rule_id"] == brief["native_rule_id"])
            })
        })
    {
        return Err("gradle_original_identity_invalid");
    }
    Ok(report)
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
