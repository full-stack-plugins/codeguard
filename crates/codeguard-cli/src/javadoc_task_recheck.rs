//! Javadoc 任务原工具复检；绑定原配置及工具，保持局部观察和可信关闭分离。
use crate::discovery::discover;
use crate::java_javadoc_scan::{NativeContext, observe_project};
use crate::java_javadoc_command::{Args, observe};
use crate::javadoc_workbench::prepare;
use codeguard_adapters::{legacy_registry, parse_unique_json};
use codeguard_runtime::{NativeObservation, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// 根据摘要绑定的原报告对指定任务运行显式JDK；不从报告选择可执行程序。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    home: Option<&Path>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let original = original(root, brief)?;
    let mode = scope_from_report(&original)?;
    let relative = brief["scope"]
        .as_str()
        .ok_or("javadoc_task_scope_invalid")?;
    let run = format!(
        "javadoc-task-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock_unavailable")?
            .as_nanos()
    );
    let mut report = json!({"schema_version":"0.2.0","observation_scope":mode,"report_type":"javadoc_task_recheck","operation":"task_verify","run_id":run,"workspace_binding":"bound","workspace_id":original["workspace_id"],"checker_id":"java.jdk.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","task_id":brief["task_id"],"task_path":relative,"task_rule":brief["native_rule_id"],"origin":brief["evidence_ref"],"scan":null,"native_observation":null,"input_bindings":[],"task_input_stable":false,"configuration_matches":false,"tool_identity_matches":false,"reason":"javadoc_recheck_incomplete"});
    let native = if mode == "explicit_file_probe" {
        let path = root
            .join(relative)
            .canonicalize()
            .map_err(|_| "javadoc_source_unavailable")?;
        if !path.starts_with(root) || !path.is_file() {
            return Err("javadoc_task_source_outside_workspace");
        }
        observe(
            &Args {
                source: path,
                java_home: home.map(Path::to_owned),
                json: true,
            },
            deadline,
            &AtomicBool::new(false),
        )
    } else {
        let registry = legacy_registry().map_err(|_| "language_registry_invalid")?;
        let discovery = discover(root, &registry, &NativeObservation);
        if !discovery.observation_complete {
            report["reason"] = json!("source_discovery_incomplete");
            return Ok(report);
        }
        if !discovery
            .languages
            .get("java")
            .is_some_and(|l| l.source_files.contains(relative))
        {
            report["reason"] = json!("javadoc_task_source_not_selected");
            return Ok(report);
        }
        observe_project(
            root,
            &BTreeSet::from([relative.to_owned()]),
            &discovery.checker_configurations,
            &NativeContext {
                manifest_sha256: &discovery.manifest_sha256,
                java_home: home,
                maven_tool: None,
                maven_repo: None,
                repo_sha256: None,
                deadline,
                cancelled: &AtomicBool::new(false),
            },
        )
    };
    report["native_observation"] = native.clone();
    let row = if mode == "explicit_file_probe" {
        json!({"path":relative,"configuration_ref":null,"configuration_sha256":null,"observation":native})
    } else {
        let Some(row) = native["files"].as_array().and_then(|r| r.first()) else {
            return Ok(report);
        };
        row.clone()
    };
    let old = original["sources"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["path"] == relative))
        .ok_or("javadoc_original_scope_invalid")?;
    report["configuration_matches"] = json!(
        row["configuration_ref"] == old["configuration_ref"]
            && row["configuration_sha256"] == old["configuration_sha256"]
    );
    report["tool_identity_matches"] = json!(
        [
            "javadoc_tool_sha256",
            "java_runtime_sha256",
            "jdk_release_sha256"
        ]
        .iter()
        .all(|key| old["native"][*key].is_string()
            && old["native"][*key] == row["observation"][*key])
    );
    let mut bindings = Vec::new();
    bindings.push(json!({"path":relative,"sha256":row["observation"]["source_sha256"],"location":"workspace"}));
    if let Some(path) = row["configuration_ref"].as_str() {
        bindings
            .push(json!({"path":path,"sha256":row["configuration_sha256"],"location":"workspace"}));
    }
    if let Some(home) = home.and_then(|h| h.canonicalize().ok()) {
        for (path, key) in [
            ("bin/javadoc", "javadoc_tool_sha256"),
            ("bin/java", "java_runtime_sha256"),
            ("release", "jdk_release_sha256"),
        ] {
            bindings.push(
                json!({"path":home.join(path),"sha256":row["observation"][key],"location":"tool"}),
            );
        }
    }
    report["input_bindings"] = json!(bindings);
    if let Ok(mut scan) = prepare(root, &json!({"native_observation":native})) {
        scan["run_id"] = report["run_id"].clone();
        report["scan"] = scan;
        report["task_input_stable"] = json!(inputs_current(root, &report));
        report["reason"] = json!("javadoc_local_recheck_policy_unverified");
    }
    Ok(report)
}

/// 只复核显式本轮输入的当前字节，不从保存路径执行程序。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    report["input_bindings"].as_array().is_some_and(|rows| {
        !rows.is_empty()
            && rows.iter().all(|row| {
                let Some(path) = row["path"].as_str() else {
                    return false;
                };
                let path = if row["location"] == "workspace" {
                    if !safe_relative_path(path) {
                        return false;
                    }
                    root.join(path)
                } else if row["location"] == "tool" && Path::new(path).is_absolute() {
                    Path::new(path).to_owned()
                } else {
                    return false;
                };
                if row["location"] == "workspace"
                    && !path.canonicalize().is_ok_and(|p| p.starts_with(root))
                {
                    return false;
                }
                read_bounded_regular_file(&path, 128 * 1024 * 1024)
                    .is_ok_and(|bytes| row["sha256"] == format!("{:x}", Sha256::digest(bytes)))
            })
    })
}

/// 分类原任务的局部状态；配置变化或工具失配不会成为修复完成。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if !valid_shape(report)
        || report["task_id"] != brief["task_id"]
        || report["task_path"] != brief["scope"]
        || report["task_rule"] != brief["native_rule_id"]
        || report["origin"] != brief["evidence_ref"]
        || (brief["observation_scope"].is_string()
            && scope_from_report(report).ok() != brief["observation_scope"].as_str())
        || report["task_input_stable"] != true
    {
        return "incomplete";
    }
    let Some(row) = report["scan"]["sources"]
        .as_array()
        .and_then(|rows| rows.first())
    else {
        return "incomplete";
    };
    if !matches!(
        row["native"]["local_status"].as_str(),
        Some("findings_observed_untrusted" | "clean_scope_unproven")
    ) {
        return "incomplete";
    }
    if brief["kind"] == "blocker" {
        return "candidate_absent_unverified_policy";
    }
    if report["tool_identity_matches"] != true {
        return "incomplete";
    }
    if report["configuration_matches"] != true {
        return "rule_coverage_requires_review";
    }
    let Some(findings) = row["findings"].as_array() else {
        return "incomplete";
    };
    if findings.iter().any(|f| f["finding_id"] == brief["task_id"]) {
        "still_present"
    } else if findings
        .iter()
        .any(|f| f["rule_id"] == brief["native_rule_id"])
    {
        "rule_coverage_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

/// 校验有限复检容器；详细原生和源码绑定由工作台导入器再次验证。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let mut keys = vec![
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
        "native_observation",
        "input_bindings",
        "task_input_stable",
        "configuration_matches",
        "tool_identity_matches",
        "reason",
    ];
    if report["schema_version"] == "0.2.0" {
        keys.push("observation_scope");
    }
    report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
        && (report["schema_version"] == "0.1.0" || scope_from_report(report).is_ok())
        && report["report_type"] == "javadoc_task_recheck"
        && report["operation"] == "task_verify"
        && report["workspace_binding"] == "bound"
        && report["checker_id"] == "java.jdk.javadoc"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["task_input_stable"].is_boolean()
        && report["configuration_matches"].is_boolean()
        && report["tool_identity_matches"].is_boolean()
        && report["input_bindings"].is_array()
}

fn original(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .ok_or("javadoc_original_reference_invalid")?;
    if !run.starts_with("javadoc-") || !safe_run_id(run) {
        return Err("javadoc_original_reference_invalid");
    }
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "javadoc_original_unavailable")?;
    if brief["evidence_ref"]["first_report_sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err("javadoc_original_changed");
    }
    let report = parse_unique_json(&bytes).map_err(|_| "javadoc_original_invalid")?;
    if report["report_type"] != "javadoc_workbench_observation"
        || report["checker_id"] != "java.jdk.javadoc"
        || report["authority"] != "local_unverified"
        || report["run_id"] != run
    {
        return Err("javadoc_original_invalid");
    }
    Ok(report)
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|p| !p.is_empty() && !matches!(p, "." | "..") && !p.chars().any(char::is_control))
}
fn safe_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// 从原报告恢复固定模式；旧报告只能属于既有项目配置模式。
pub(crate) fn original_scope(root: &Path, brief: &Value) -> Result<&'static str, &'static str> {
    scope_from_report(&original(root, brief)?)
}
fn scope_from_report(report: &Value) -> Result<&'static str, &'static str> {
    if report["schema_version"] == "0.1.0" {
        return Ok("configured_project_probe");
    }
    match report["observation_scope"].as_str() {
        Some("explicit_file_probe") => Ok("explicit_file_probe"),
        Some("configured_project_probe") => Ok("configured_project_probe"),
        _ => Err("javadoc_observation_scope_invalid"),
    }
}
