//! 从严格原生模型选择启用的官方Javadoc任务。
use crate::{GradleCheckerModel, GradleJavadocTaskPlan, parse_gradle_checker_model};
/// 重新验证模型并返回原生任务的完整路径；空集合不证明其它文档引擎不存在。
/// 参数为当前模型；返回至多128项局部计划，配置范围与规则仍待调用方核验。
pub fn plan_gradle_javadoc_tasks(
    model: &GradleCheckerModel,
) -> Result<Vec<GradleJavadocTaskPlan>, &'static str> {
    let bytes = serde_json::to_vec(model).map_err(|_| "gradle_model_invalid")?;
    let model = parse_gradle_checker_model(&bytes)?;
    let mut result = Vec::new();
    for project in model.projects {
        if !project
            .plugins
            .iter()
            .any(|id| matches!(id.as_str(), "java" | "java-library"))
        {
            continue;
        }
        for task in project.tasks {
            if task.enabled && task.implementation == "org.gradle.api.tasks.javadoc.Javadoc" {
                if result.len() >= 128 {
                    return Err("gradle_javadoc_task_budget_exceeded");
                }
                let prefix = if project.path == ":" {
                    ""
                } else {
                    project.path.as_str()
                };
                result.push(GradleJavadocTaskPlan {
                    project_path: project.path.clone(),
                    task_path: format!("{prefix}:{}", task.name),
                });
            }
        }
    }
    Ok(result)
}
