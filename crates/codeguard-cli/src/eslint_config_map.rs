//! 显式monorepo上下文映射，只选择原调用参数，不推断生效配置或批准。
use crate::{
    eslint_lint_arguments::EslintLintArguments, eslint_project_context::EslintProjectContext,
};
use codeguard_runtime::read_bounded_regular_file;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
    time::Instant,
};
/// 有版本的显式子项目映射；重复字段、未知字段和范围跳转均拒绝。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EslintConfigMap {
    schema_version: String,
    projects: Vec<EslintProjectContext>,
}
impl EslintConfigMap {
    /// 读取有界、物理路径一致的映射与摘要，并验证所有原配置/cwd归属。
    pub(crate) fn load(
        root: &Path,
        path: &Path,
        deadline: Instant,
    ) -> Result<(Self, String), &'static str> {
        budget(deadline)?;
        let invalid = "eslint_config_map_invalid";
        if path.canonicalize().ok().as_deref() != Some(path) {
            return Err(invalid);
        }
        let bytes = read_bounded_regular_file(path, 64 * 1024).map_err(|_| invalid)?;
        let map: Self = serde_json::from_slice(&bytes).map_err(|_| invalid)?;
        if map.schema_version != "1.0" || map.projects.is_empty() || map.projects.len() > 128 {
            return Err(invalid);
        }
        let mut roots = BTreeSet::new();
        for project in &map.projects {
            budget(deadline)?;
            if !safe_relative(&project.root, true)
                || !safe_relative(&project.cwd, true)
                || !safe_relative(&project.config, false)
                || !roots.insert(&project.root)
            {
                return Err(invalid);
            }
            for path in [resolve(root, &project.root), resolve(root, &project.cwd)] {
                if !path.is_dir() || path.canonicalize().ok().as_deref() != Some(path.as_path()) {
                    return Err(invalid);
                }
            }
            let config = root.join(&project.config);
            if config.canonicalize().ok().as_deref() != Some(config.as_path())
                || read_bounded_regular_file(&config, 1024 * 1024).is_err()
            {
                return Err(invalid);
            }
        }
        budget(deadline)?;
        Ok((map, format!("{:x}", Sha256::digest(bytes))))
    }
    /// 按路径组件选择最深显式项目；未命中保留调用者原配置和cwd。
    pub(crate) fn select(
        &self,
        root: &Path,
        source: &Path,
        args: &EslintLintArguments,
    ) -> (EslintLintArguments, String, bool) {
        let mut selected = args.clone();
        selected.source = source.to_path_buf();
        let best = self
            .projects
            .iter()
            .filter(|p| source.starts_with(resolve(root, &p.root)))
            .max_by_key(|p| {
                if p.root == "." {
                    0
                } else {
                    Path::new(&p.root).components().count()
                }
            });
        if let Some(project) = best {
            selected.config = Some(root.join(&project.config));
            selected.cwd = Some(resolve(root, &project.cwd));
            (selected, project.root.clone(), true)
        } else {
            (selected, ".".into(), false)
        }
    }
    /// 返回全部显式配置路径，纳入本輪目录输入前后复核。
    pub(crate) fn configs(&self, root: &Path) -> Vec<std::path::PathBuf> {
        self.projects.iter().map(|p| root.join(&p.config)).collect()
    }
}
fn budget(deadline: Instant) -> Result<(), &'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Err("request_cancelled")
    } else if Instant::now() >= deadline {
        Err("request_deadline_exceeded")
    } else {
        Ok(())
    }
}
fn resolve(root: &Path, value: &str) -> std::path::PathBuf {
    if value == "." {
        root.into()
    } else {
        root.join(value)
    }
}
fn safe_relative(value: &str, dot: bool) -> bool {
    (dot && value == ".")
        || (!value.is_empty()
            && value.len() <= 4096
            && !value.chars().any(char::is_control)
            && !value.contains('\\')
            && !value
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            && Path::new(value)
                .components()
                .all(|c| matches!(c, Component::Normal(_))))
}
pub(crate) fn digest(path: &Path) -> Result<String, &'static str> {
    read_bounded_regular_file(path, 64 * 1024)
        .map(|b| format!("{:x}", Sha256::digest(b)))
        .map_err(|_| "eslint_config_map_invalid")
}
