//! 对 Checkstyle 准备任务重跑原生检查，恢复只作为未验证观察。
use crate::checkstyle_workbench::prepare;
use crate::java_checkstyle_command::observe;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// 在既有截止时间内对准备任务范围运行显式工具；不从历史记录选择可执行程序。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Value {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let relative = brief["preparation_guidance"]["affected_paths"][0].as_str();
    let mut report = json!({"schema_version":"0.1.0","report_type":"checkstyle_preparation_recheck","run_id":format!("checkstyle-preparation-verify-{}-{nanos}",std::process::id()),
        "workspace_binding":"bound","workspace_id":brief["evidence_workspace_id"],"checker_id":"java.checkstyle.preparation",
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","local_status":"incomplete","reason":"prerequisites_missing",
        "target":{"task_id":brief["task_id"],"path":relative},"scan":null,"task_input_stable":null});
    let Some(relative) = relative.filter(|s| safe_path(s)) else {
        report["reason"] = json!("task_scope_invalid");
        return report;
    };
    let source = root.join(relative);
    if !source.canonicalize().is_ok_and(|p| p.starts_with(root)) {
        report["reason"] = json!("task_source_outside_workspace");
        return report;
    }
    if !["--java-tool", "--checkstyle-jar", "--config"]
        .iter()
        .all(|k| options.contains_key(*k))
    {
        return report;
    }
    let observed = match observe(&source.to_string_lossy(), options, deadline) {
        Ok(v) => v,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    match prepare(root, &observed, &observed["input_bindings"]) {
        Ok(mut scan) => {
            scan["run_id"] = report["run_id"].clone();
            report["scan"] = scan;
            report["local_status"] = json!("observed");
            report["reason"] = json!("original_selection_and_quality_policy_unverified");
            report["task_input_stable"] = json!(inputs_current(&report));
        }
        Err(reason) => report["reason"] = json!(reason),
    }
    report
}

/// 分类准备复检，仅同范围完整局部观察可成为恢复线索，所有结果保留任务开放。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if !valid_shape(report)
        || brief["kind"] != "blocker"
        || report["target"]["task_id"] != brief["task_id"]
        || (!brief["evidence_workspace_id"].is_null()
            && report["workspace_id"] != brief["evidence_workspace_id"])
    {
        return "incomplete";
    }
    if report["local_status"] == "observed" {
        if report["task_input_stable"] == true {
            "environment_restored_unverified_policy"
        } else {
            "incomplete"
        }
    } else if matches!(
        report["reason"].as_str(),
        Some(
            "prerequisites_missing"
                | "configuration_unavailable"
                | "java_tool_unavailable"
                | "checkstyle_jar_unavailable"
                | "checkstyle_configuration_context_unresolved"
        )
    ) {
        "still_blocked"
    } else {
        "incomplete"
    }
}

/// 保存收据前重读本轮冻结输入；失败观察不虚构执行输入证明。
pub(crate) fn inputs_current(report: &Value) -> bool {
    if report["scan"].is_null() {
        return report["local_status"] == "incomplete";
    }
    [
        ("source", 16 * 1024 * 1024),
        ("config", 1024 * 1024),
        ("java", 128 * 1024 * 1024),
        ("jar", 128 * 1024 * 1024),
    ]
    .iter()
    .all(|(key, limit)| {
        let input = &report["scan"]["inputs"][*key];
        input["path"].as_str().is_some_and(|p| {
            read_bounded_regular_file(Path::new(p), *limit)
                .ok()
                .is_some_and(|b| input["sha256"] == format!("{:x}", Sha256::digest(b)))
        })
    })
}

/// 校验局部复检容器与范围；参数是未授权报告，不能据此关闭任务。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "run_id",
        "workspace_binding",
        "workspace_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "local_status",
        "reason",
        "target",
        "scan",
        "task_input_stable",
    ];
    report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && report["run_id"].as_str().is_some_and(|s| {
            s.strip_prefix("checkstyle-preparation-verify-")
                .is_some_and(|s| {
                    let parts: Vec<_> = s.split('-').collect();
                    parts.len() == 2
                        && parts.iter().all(|p| {
                            !p.is_empty()
                                && p.bytes().all(|b| b.is_ascii_digit())
                                && p.parse::<u128>().is_ok_and(|v| v > 0)
                        })
                })
        })
        && report["workspace_id"].as_str().is_some_and(|s| {
            s.strip_prefix("ws-").is_some_and(|s| {
                s.len() == 32
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        })
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "checkstyle_preparation_recheck"
        && report["workspace_binding"] == "bound"
        && report["checker_id"] == "java.checkstyle.preparation"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["reason"].as_str().is_some_and(|s| {
            !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        && report["target"]
            .as_object()
            .is_some_and(|o| o.len() == 2 && o.contains_key("task_id") && o.contains_key("path"))
        && report["target"]["task_id"].as_str().is_some_and(|s| {
            s.strip_prefix("CG-B-").is_some_and(|s| {
                s.len() == 32
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        })
        && report["target"]["path"].as_str().is_some_and(safe_path)
        && ((report["local_status"] == "incomplete"
            && report["scan"].is_null()
            && report["task_input_stable"].is_null())
            || (report["local_status"] == "observed"
                && report["scan"].is_object()
                && report["task_input_stable"].is_boolean()
                && report["scan"]["run_id"] == report["run_id"]
                && report["scan"]["workspace_id"] == report["workspace_id"]
                && report["scan"]["source_path"] == report["target"]["path"]))
}
fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('/')
        && !s.contains(['\\', ':'])
        && s.split('/')
            .all(|p| !matches!(p, "" | "." | "..") && !p.chars().any(char::is_control))
}
