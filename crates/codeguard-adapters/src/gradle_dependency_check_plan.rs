use crate::{GradleCheckerModel, GradleDependencyCheckTaskPlan, parse_gradle_checker_model};
use std::collections::BTreeSet;

/// 选择显式OWASP任务；参数为原生模型和完整任务路径，返回经插件/类型/启用状态核验的计划。
/// 不自动执行同名普通任务，不将根插件身份继承给子项目。
pub fn plan_gradle_dependency_check_tasks(
    model: &GradleCheckerModel,
    requested: &[String],
) -> Result<Vec<GradleDependencyCheckTaskPlan>, &'static str> {
    let model = parse_gradle_checker_model(
        &serde_json::to_vec(model).map_err(|_| "gradle_model_invalid")?,
    )?;
    if requested.is_empty() || requested.len() > 128 {
        return Err("gradle_owasp_task_selection_invalid");
    }
    let mut seen = BTreeSet::new();
    let mut plans = Vec::new();
    for path in requested {
        if !seen.insert(path) {
            return Err("gradle_owasp_task_selection_duplicate");
        }
        let mut selected = None;
        for project in &model.projects {
            let prefix = if project.path == ":" {
                ""
            } else {
                project.path.as_str()
            };
            for task in project.dependency_check_tasks() {
                if format!("{prefix}:{}", task.name) == *path {
                    selected = Some(GradleDependencyCheckTaskPlan {
                        project_path: project.path.clone(),
                        task_path: path.clone(),
                        implementation: task.implementation.clone(),
                    });
                }
            }
        }
        plans.push(selected.ok_or("gradle_owasp_task_unavailable")?);
    }
    Ok(plans)
}
