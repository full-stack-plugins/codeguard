//! Python CVE 稳定任务的原工具复检解释；覆盖和漏洞源不自获批准。

use crate::python_cve_command::{bounded_build_root, input_identity, lock_inputs};
use serde_json::Value;
use std::path::Path;

/// 只接受本地未验证的 Python CVE 扫描包装，不把报告字段当成批准。
pub(crate) fn valid_shape(report: &Value) -> bool {
    let scan = &report["scan"];
    report.as_object().is_some_and(|object| {
        object.len() == 14
            && [
                "schema_version",
                "report_type",
                "workspace_binding",
                "workspace_id",
                "run_id",
                "checker_id",
                "authority",
                "coverage_proven",
                "delivery_decision",
                "build_root",
                "manifest_state",
                "manifest_sha256",
                "lock_inputs",
                "scan",
            ]
            .iter()
            .all(|key| object.contains_key(*key))
    }) && report["schema_version"] == "0.1.0"
        && report["report_type"] == "python_cve_workbench_observation"
        && report["workspace_binding"] == "bound"
        && report["checker_id"] == "python.pip_audit"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && scan["schema_version"] == "0.1.0"
        && scan["report_type"] == "python_cve_local_observation"
        && scan["checker_id"] == "python.pip_audit"
        && scan["coverage_proven"] == false
        && scan["advisory_coverage"] == "not_evaluated"
        && scan["delivery_decision"] == "not_evaluated"
}

/// 复检事件写入前重新确认项目清单和整组标准锁未变化。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    if !valid_shape(report) {
        return false;
    }
    let Some(build_root) = report["build_root"].as_str() else {
        return false;
    };
    let Ok(project) = bounded_build_root(root, build_root) else {
        return false;
    };
    let manifest = input_identity(&project.join("pyproject.toml"), 2 * 1024 * 1024);
    report["manifest_state"] == manifest.0
        && report["manifest_sha256"] == manifest.1
        && lock_inputs(&project).is_ok_and(|locks| report["lock_inputs"] == locks)
}

/// 原生复检只追加观察；完整依赖图、漏洞源与策略未经核验时保持开放。
pub(crate) fn classify(brief: &Value, report: &Value) -> &'static str {
    if brief["kind"] != "blocker"
        || brief["checker_id"] != "python.pip_audit"
        || brief["reason_code"] != "python_cve_coverage_unverified"
        || brief["build_root"] != report["build_root"]
        || !valid_shape(report)
    {
        return "incomplete";
    }
    "still_blocked"
}
