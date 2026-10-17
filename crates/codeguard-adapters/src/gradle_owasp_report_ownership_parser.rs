use crate::{
    GradleCheckerModel, GradleOwaspReportOwnership, parse_unique_json,
    plan_gradle_dependency_check_tasks,
};
use std::collections::BTreeSet;

/// 验证本轮原生输出归属；参数为有界JSON、当前模型及原请求路径，返回逐任务唯一报告位置。
/// 拒绝不匹配/重复任务、输出别名/越界和缺失归属，不根据报告文件名推断扫描完成。
pub fn parse_gradle_owasp_report_ownership(
    raw: &[u8],
    model: &GradleCheckerModel,
    requested: &[String],
) -> Result<Vec<GradleOwaspReportOwnership>, &'static str> {
    if raw.is_empty() || raw.len() > 1024 * 1024 {
        return Err("gradle_owasp_ownership_budget_invalid");
    }
    let doc = parse_unique_json(raw).map_err(|_| "gradle_owasp_ownership_json_invalid")?;
    let object = doc
        .as_object()
        .ok_or("gradle_owasp_ownership_shape_invalid")?;
    if object.len() != 3
        || doc["schema_version"] != "0.1.0"
        || doc["report_type"] != "gradle_owasp_report_ownership"
    {
        return Err("gradle_owasp_ownership_protocol_invalid");
    }
    let plans = plan_gradle_dependency_check_tasks(model, requested)?;
    let rows = doc["tasks"]
        .as_array()
        .ok_or("gradle_owasp_ownership_shape_invalid")?;
    if rows.len() != plans.len() {
        return Err("gradle_owasp_ownership_incomplete");
    }
    let mut paths = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut result = Vec::new();
    for value in rows {
        let row: GradleOwaspReportOwnership = serde_json::from_value(value.clone())
            .map_err(|_| "gradle_owasp_ownership_shape_invalid")?;
        if !plans.iter().any(|plan| {
            plan.project_path == row.project_path
                && plan.task_path == row.task_path
                && plan.implementation == row.implementation
        }) || !tasks.insert(row.task_path.clone())
        {
            return Err("gradle_owasp_ownership_task_mismatch");
        }
        if row.report_project_name.is_empty()
            || row.report_project_name.len() > 256
            || row.report_project_name.chars().any(char::is_control)
        {
            return Err("gradle_owasp_project_name_invalid");
        }
        if row.report_path.len() > 256
            || !row.report_path.ends_with("/dependency-check-report.json")
            || row.report_path.split('/').any(|part| {
                part.is_empty()
                    || matches!(part, "." | "..")
                    || part.contains(['\\', ':'])
                    || part.chars().any(char::is_control)
            })
            || !paths.insert(row.report_path.clone())
        {
            return Err("gradle_owasp_report_scope_invalid");
        }
        result.push(row);
    }
    Ok(result)
}
