//! 单个Gradle子项目的原生检查配置观察。
use crate::gradle_checker_task::GradleCheckerTask;
use serde::{Deserialize, Serialize};
/// 子项目路径、输出范围、插件和任务事实；不声明检查已完成。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GradleProjectCheckerModel {
    /// Gradle项目身份，例如根项目:或:app。
    pub path: String,
    /// 相对受检构建根的子项目目录。
    pub project_dir: String,
    /// 相对受检构建根的Gradle输出目录，越界目录须拒绝。
    pub build_dir: String,
    /// 实际应用的已观察插件ID；未出现不证明其它检查器不存在。
    pub plugins: Vec<String>,
    /// 当前已注册的相关任务，包含禁用或仅名称相似的任务。
    pub tasks: Vec<GradleCheckerTask>,
}
impl GradleProjectCheckerModel {
    /// 返回适用OWASP插件中已启用的官方任务，尚需报告/依赖图及数据库验收。
    pub fn dependency_check_tasks(&self) -> Vec<&GradleCheckerTask> {
        if !self
            .plugins
            .iter()
            .any(|id| id == "org.owasp.dependencycheck")
        {
            return Vec::new();
        }
        self.tasks
            .iter()
            .filter(|task| task.is_dependency_check())
            .collect()
    }
}
