//! pip-audit 标准锁项目模式的字面参数计划；不运行工具或授予漏洞库可信身份。

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// 使用显式工具、项目根和私有缓存调用 pip-audit 的候选计划。
pub struct PipAuditCommand {
    /// 待由执行器核对身份的原生工具绝对路径。
    pub tool: PathBuf,
    /// 含 pyproject 和标准 pylock 的项目绝对根路径。
    pub project_root: PathBuf,
    /// 项目外私有 HTTP 缓存路径，不得污染源码。
    pub cache: PathBuf,
}

impl PipAuditCommand {
    /// 返回固定项目锁审计参数；执行层仍须冻结输入、清理环境并核验来源和报告。
    pub fn args(&self) -> Result<Vec<OsString>, &'static str> {
        for path in [&self.tool, &self.project_root, &self.cache] {
            if !safe_absolute_path(path) {
                return Err("pip_audit_path_invalid");
            }
        }
        if self.cache.starts_with(&self.project_root) {
            return Err("pip_audit_cache_inside_project");
        }
        if self.tool == self.project_root || self.cache == self.tool {
            return Err("pip_audit_path_invalid");
        }
        Ok(vec![
            "--locked".into(),
            self.project_root.as_os_str().to_owned(),
            "--strict".into(),
            "--format".into(),
            "json".into(),
            "--aliases".into(),
            "on".into(),
            "--desc".into(),
            "off".into(),
            "--progress-spinner".into(),
            "off".into(),
            "--vulnerability-service".into(),
            "pypi".into(),
            "--cache-dir".into(),
            self.cache.as_os_str().to_owned(),
            "--timeout".into(),
            "15".into(),
        ])
    }
}

fn safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|text| text.len() <= 4096 && !text.chars().any(char::is_control))
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}
