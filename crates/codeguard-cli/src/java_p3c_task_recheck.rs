//! P3C项目身份与修复复验的真实纯分类逻辑；不执行工具或授予批准。
use serde_json::Value;
/// 检查明确支持的项目报告身份；独立local_feedback不属于此入口。
pub(crate) fn valid_shape(report: &Value) -> bool {
    report["report_type"] == "java_p3c_project_observation"
        && matches!(
            report["schema_version"].as_str(),
            Some("0.2.0" | "0.3.0" | "0.4.0")
        )
        && (report["schema_version"] != "0.4.0" || report["task_input_stable"].is_boolean())
        && report["checker_id"] == "java.maven.p3c"
        && report["coverage_proven"] == false
}
/// 核对保存事件的原生分类；文件摘要、工作区与输入证明仍由调用方独立核验。
pub(crate) fn valid_event(brief: &Value, report: &Value, observation: &Value) -> bool {
    valid_shape(report) && observation == classify(brief, report)
}
pub(crate) fn has_projectable_findings(report: &Value) -> bool {
    report["findings"]
        .as_array()
        .is_some_and(|findings| !findings.is_empty())
        && (report["local_status"] == "findings_observed_untrusted"
            || (report["local_status"] == "incomplete"
                && report["reason"] == "native_execution_failed"))
}

/// 复用原任务分类：阳性仍保留，无发现仍需规则覆盖审阅，不关闭任务。
pub(crate) fn classify(brief: &Value, scan: &Value) -> &'static str {
    if !valid_shape(scan) || scan["task_input_stable"] == false {
        return "incomplete";
    }
    let Some(files) = scan["files"].as_array() else {
        return "incomplete";
    };
    let valid_file = |path: &str| {
        files
            .iter()
            .find(|file| file["path"] == path)
            .is_some_and(|file| {
                file["configuration"] == "configured"
                    && file["reason"] == "native_probe_returned"
                    && matches!(
                        file["observation"]["local_status"].as_str(),
                        Some("findings_observed_untrusted" | "clean_scope_unproven")
                    )
            })
    };
    if brief["kind"] == "blocker" {
        return if brief["affected_paths"].as_array().is_some_and(|paths| {
            !paths.is_empty()
                && paths
                    .iter()
                    .all(|path| path.as_str().is_some_and(valid_file))
        }) {
            "environment_restored_unverified_policy"
        } else {
            "still_blocked"
        };
    }
    if brief["kind"] != "finding" {
        return "incomplete";
    }
    let Some(path) = brief["scope"].as_str() else {
        return "incomplete";
    };
    let partial_positive = files.iter().any(|file| {
        file["path"] == path
            && file["configuration"] == "configured"
            && file["reason"] == "native_probe_returned"
            && has_projectable_findings(&file["observation"])
    });
    if !valid_file(path) && !partial_positive {
        return "incomplete";
    }
    if scan["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|finding| {
            finding["finding_id"] == brief["task_id"]
                && finding["path"] == path
                && finding["rule_id"] == brief["native_rule_id"]
        })
    }) {
        "still_present"
    } else if valid_file(path) {
        "rule_coverage_requires_review"
    } else {
        "incomplete"
    }
}

/// 将本次输入稳定性附加到当前原生扫描，使用独立复验wire身份。
pub(crate) fn bind_task_input(scan: &mut Value, stable: bool) {
    if scan["schema_version"] == "0.3.0" && valid_shape(scan) {
        scan["schema_version"] = serde_json::json!("0.4.0");
        scan["task_input_stable"] = serde_json::json!(stable);
    }
}
/// 当前复验外层身份；不改变分类或授权状态。
pub(crate) fn preview_version(scan: &Value) -> &'static str {
    if scan["schema_version"] == "0.4.0" && valid_shape(scan) {
        "0.40.0"
    } else {
        "0.6.0"
    }
}
/// 绑定文件反馈初始信封；项目观察在调用方实际执行后填入。
pub(crate) fn file_feedback(path: &str) -> Value {
    serde_json::json!({"schema_version":"0.2.0","report_type":"java_p3c_file_feedback",
        "operation":"lint","language":"java","checker_id":"java.maven.p3c",
        "command_status":"incomplete","exit_code":3,"scope":"single_file",
        "path":path,"project_observation":null,
        "workbench":{"status":"backlog_update_failed","reason":"not_run"},"next":null,
        "coverage_proven":false,"delivery_decision":"not_evaluated"})
}
