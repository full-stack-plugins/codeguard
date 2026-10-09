//! 原生Gradle模型中Javadoc任务的精确调用位置。
use serde::Serialize;
/// 只表示可尝试的官方任务身份，不证明doclint配置或注释质量通过。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GradleJavadocTaskPlan {
    /// 原生子项目身份。
    pub project_path: String,
    /// 完整Gradle任务路径，避免同名任务的模糊选择。
    pub task_path: String,
}
