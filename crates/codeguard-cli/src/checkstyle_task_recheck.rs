//! Checkstyle 稳定任务的局部原生复检，所有结果均保留开放状态。
use crate::checkstyle_workbench::prepare;
use crate::java_checkstyle_command::observe;
use codeguard_adapters::checkstyle_comment_rule_bindings;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// 按已核对任务范围复检显式工具；参数为根目录、事实简报、显式选项和统一截止时间。
/// 返回局部报告，不从任务或观察中自动选择可执行程序。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Value {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let run_id = format!("checkstyle-task-{}-{nanos}", std::process::id());
    let mut report = json!({"schema_version":"0.1.0","report_type":"checkstyle_task_recheck","run_id":run_id,
        "workspace_binding":"bound","workspace_id":brief["evidence_workspace_id"],"checker_id":"java.checkstyle",
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "local_status":"incomplete","reason":"prerequisites_missing","scan":null,
        "target":{"task_id":brief["task_id"],"path":brief["scope"],"rule_id":brief["native_rule_id"],"checker_class":brief["checkstyle_guidance"]["checker_class"]},
        "current_rule_class":null,"tool_identity_matches":false,"task_input_stable":null});
    if !["--java-tool", "--checkstyle-jar", "--config"]
        .iter()
        .all(|k| options.contains_key(*k))
    {
        return report;
    }
    let Some(relative) = brief["scope"].as_str() else {
        report["reason"] = json!("task_scope_invalid");
        return report;
    };
    let source = root.join(relative);
    if !source.canonicalize().is_ok_and(|p| p.starts_with(root)) {
        report["reason"] = json!("task_source_outside_workspace");
        return report;
    }
    let observed = match observe(&source.to_string_lossy(), options, deadline) {
        Ok(value) => value,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    let inputs = &observed["input_bindings"];
    let old = &brief["checkstyle_guidance"]["original_tool_inputs"];
    report["tool_identity_matches"] = json!(
        ["java", "jar"]
            .iter()
            .all(|k| inputs[*k]["sha256"] == old[*k]["sha256"] && inputs[*k]["sha256"].is_string())
    );
    let config = Path::new(options.get("--config").expect("已核验参数"));
    if let Ok(bytes) = read_bounded_regular_file(config, 1024 * 1024) {
        if let Some(bindings) = checkstyle_comment_rule_bindings(&bytes, "10.21.4") {
            report["current_rule_class"] = brief["native_rule_id"]
                .as_str()
                .and_then(|r| bindings.get(r))
                .map_or(Value::Null, |b| json!(b.checker_class));
        }
    }
    match prepare(root, &observed, inputs) {
        Ok(mut scan) => {
            scan["run_id"] = report["run_id"].clone();
            report["workspace_id"] = scan["workspace_id"].clone();
            report["scan"] = scan;
            report["local_status"] = json!("observed");
            report["reason"] = json!("project_configuration_and_quality_policy_unverified");
            report["task_input_stable"] = json!(inputs_current(&report));
        }
        Err(reason) => report["reason"] = json!(reason),
    }
    report
}

/// 对任务身份、规则和工具一致性进行局部分类；返回值不含正式关闭结论。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if report["report_type"] != "checkstyle_task_recheck"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["target"]["task_id"] != brief["task_id"]
        || report["target"]["path"] != brief["scope"]
        || report["target"]["rule_id"] != brief["native_rule_id"]
        || report["target"]["checker_class"] != brief["checkstyle_guidance"]["checker_class"]
        || report["local_status"] != "observed"
        || report["tool_identity_matches"] != true
        || report["task_input_stable"] != true
    {
        return "incomplete";
    }
    if report["current_rule_class"] != report["target"]["checker_class"] {
        return "rule_coverage_requires_review";
    }
    let Some(findings) = report["scan"]["findings"].as_array() else {
        return "incomplete";
    };
    if findings.iter().any(|f| f["finding_id"] == brief["task_id"]) {
        "still_present"
    } else if !brief["checkstyle_guidance"]["original_configuration_input"]["sha256"].is_string()
        || report["scan"]["inputs"]["config"]["sha256"]
            != brief["checkstyle_guidance"]["original_configuration_input"]["sha256"]
    {
        // 检查类相同不能证明范围相同；原配置变更需要复核，不能充当源码修复。
        "rule_coverage_requires_review"
    } else if findings
        .iter()
        .any(|f| f["rule_id"] == brief["native_rule_id"])
    {
        "rule_coverage_requires_review"
    } else {
        "candidate_absent_unverified_policy"
    }
}

/// 在保存复检收据前重新核对冻结输入；失败观察没有执行输入，不虚构稳定性证明。
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
                .is_some_and(|bytes| input["sha256"] == format!("{:x}", Sha256::digest(bytes)))
        })
    })
}

/// 校验复检容器的有限结构及未授权状态；参数为保存的 JSON，不证明来源可信。
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
        "scan",
        "target",
        "current_rule_class",
        "tool_identity_matches",
        "task_input_stable",
    ];
    report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "checkstyle_task_recheck"
        && report["workspace_binding"] == "bound"
        && report["checker_id"] == "java.checkstyle"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["tool_identity_matches"].is_boolean()
        && report["reason"].as_str().is_some_and(|s| {
            !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        && report["target"].as_object().is_some_and(|o| {
            o.len() == 4
                && ["task_id", "path", "rule_id", "checker_class"]
                    .iter()
                    .all(|k| {
                        o.get(*k)
                            .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                    })
        })
        && ((report["local_status"] == "incomplete"
            && report["scan"].is_null()
            && report["task_input_stable"].is_null())
            || (report["local_status"] == "observed"
                && report["scan"].is_object()
                && report["scan"]["run_id"] == report["run_id"]
                && report["scan"]["workspace_id"] == report["workspace_id"]
                && report["task_input_stable"].is_boolean()))
}
