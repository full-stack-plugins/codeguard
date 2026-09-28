//! npm稳定完整性任务的原工具复检，不从零发现推导漏洞覆盖或关闭。
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, time::Instant};

/// 按持久任务构建根复检；只允许明确的原生上下文，不接受任务文本指令。
#[cfg(unix)]
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let scope = brief["scope"].as_str().ok_or("task_scope_invalid")?;
    if scope != "."
        && (scope.is_empty()
            || scope.contains('\\')
            || Path::new(scope)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_))))
    {
        return Err("task_scope_invalid");
    }
    let mut argv = vec![
        "typescript".into(),
        root.join(scope).to_string_lossy().into_owned(),
        "--workspace".into(),
        root.to_string_lossy().into_owned(),
    ];
    for (key, value) in options {
        argv.extend([key.clone(), value.clone()]);
    }
    let args = crate::npm_audit_arguments::NpmAuditArguments::parse(&argv)
        .map_err(|_| "npm_recheck_options_invalid")?;
    crate::npm_audit_command::observe_for_task(root, &args, deadline)
}

#[cfg(not(unix))]
pub(crate) fn run(
    _root: &Path,
    _brief: &Value,
    _options: &BTreeMap<String, String>,
    _deadline: Instant,
) -> Result<Value, &'static str> {
    Err("npm_recheck_platform_unavailable")
}

/// 复用同步器的严格报告及当前清单/锁身份核验。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    crate::work_sync::valid_npm_observation(root, report)
}

/// 本地一致的审计仍保留完整性阻塞，工具失败与输入变化为未完成。
pub(crate) fn classify(root: &Path, brief: &Value, report: &Value) -> &'static str {
    if !matches_task(brief, report)
        || !inputs_current(root, report)
        || report["diagnostic_reason"] != "npm_database_and_policy_unverified"
    {
        "incomplete"
    } else {
        "still_blocked"
    }
}

/// 原生复检必须属于同一稳定义务和构建根，失败状态也不能跨任务复用。
pub(crate) fn matches_task(brief: &Value, report: &Value) -> bool {
    brief["checker_id"] == "node.npm.audit"
        && brief["kind"] == "blocker"
        && brief["reason_code"] == "npm_audit_coverage_unverified"
        && report["checker_id"] == "node.npm.audit"
        && report["report_type"] == "npm_cve_workbench_observation"
        && brief["scope"].is_string()
        && brief["scope"] == report["build_root"]
}

/// 只有结构有效但本轮清单/锁已变或不可读时，历史复检才按过期处理。
pub(crate) fn inputs_stale(root: &Path, report: &Value) -> bool {
    matches!(
        crate::work_sync::inspect_npm_observation(root, report),
        Err("npm_report_input_changed"
            | "npm_report_input_unavailable"
            | "npm_report_input_invalid")
    )
}
