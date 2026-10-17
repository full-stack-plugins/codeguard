//! 原生Gradle模型严格解析；不执行脚本、不签发覆盖或漏洞通过结论。
use crate::{gradle_checker_model::GradleCheckerModel, strict_json::parse_unique_json};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
/// 解析有界原生模型，拒绝重复字段、重复项目/任务、越界输出、缺父项目和未覆盖composite build。
/// 参数为本轮模型JSON字节；返回限定事实，调用方仍须核对进程和输入身份。
pub fn parse_gradle_checker_model(bytes: &[u8]) -> Result<GradleCheckerModel, &'static str> {
    if bytes.is_empty() || bytes.len() > 1024 * 1024 {
        return Err("gradle_model_size_invalid");
    }
    let value = parse_unique_json(bytes).map_err(|_| "gradle_model_json_invalid")?;
    let model: GradleCheckerModel =
        serde_json::from_value(value).map_err(|_| "gradle_model_shape_invalid")?;
    if model.schema_version != "0.1.0" || model.report_type != "gradle_checker_model" {
        return Err("gradle_model_protocol_unsupported");
    }
    if model.gradle_version.len() > 32
        || !(2..=3).contains(&model.gradle_version.split('.').count())
        || !model
            .gradle_version
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("gradle_version_unresolved");
    }
    if model.included_build_count != 0 {
        return Err("gradle_composite_build_unresolved");
    }
    if model.projects.is_empty() || model.projects.len() > 128 {
        return Err("gradle_project_budget_invalid");
    }
    let mut ids = BTreeSet::new();
    let mut dirs = BTreeSet::new();
    for project in &model.projects {
        if !valid_project_path(&project.path)
            || !relative(&project.project_dir, true)
            || !relative(&project.build_dir, false)
        {
            return Err("gradle_project_scope_invalid");
        }
        if !ids.insert(project.path.as_str()) || !dirs.insert(project.project_dir.as_str()) {
            return Err("gradle_project_identity_duplicate");
        }
        if (project.path == ":") != (project.project_dir == ".") {
            return Err("gradle_root_identity_invalid");
        }
        if project.plugins.len() > 6 || project.tasks.len() > 64 {
            return Err("gradle_checker_budget_invalid");
        }
        let mut plugins = BTreeSet::new();
        let mut tasks = BTreeSet::new();
        for plugin in &project.plugins {
            if !matches!(
                plugin.as_str(),
                "java"
                    | "java-library"
                    | "org.owasp.dependencycheck"
                    | "checkstyle"
                    | "pmd"
                    | "com.github.spotbugs"
            ) || !plugins.insert(plugin)
            {
                return Err("gradle_plugin_identity_invalid");
            }
        }
        for task in &project.tasks {
            if !safe_text(&task.name, 256)
                || task.name.contains([':', '/', '\\'])
                || !safe_text(&task.implementation, 512)
                || !tasks.insert(&task.name)
            {
                return Err("gradle_task_identity_invalid");
            }
        }
    }
    if !ids.contains(":") {
        return Err("gradle_root_project_missing");
    }
    for project in &model.projects {
        if project.path == ":" {
            continue;
        }
        let parent = project
            .path
            .rsplit_once(':')
            .map(|(p, _)| if p.is_empty() { ":" } else { p })
            .ok_or("gradle_project_scope_invalid")?;
        if !ids.contains(parent) {
            return Err("gradle_parent_project_missing");
        }
    }
    Ok(model)
}
fn valid_project_path(path: &str) -> bool {
    path == ":"
        || (safe_text(path, 1024)
            && path.starts_with(':')
            && path[1..]
                .split(':')
                .all(|s| !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\'])))
}
fn safe_text(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}
fn relative(path: &str, root: bool) -> bool {
    (root && path == ".")
        || (safe_text(path, 4096)
            && !path.contains('\\')
            && !path.contains(':')
            && !path.starts_with('/')
            && path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
            && Path::new(path)
                .components()
                .all(|c| matches!(c, Component::Normal(_))))
}
