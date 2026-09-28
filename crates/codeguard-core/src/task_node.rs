//! 检查计划中的单个任务及其显式依赖、互斥资源。

/// 不含执行实现的任务节点；资源名由计划器按构建根规范化后提供。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskNode {
    /// 在本次计划中唯一的稳定任务 ID。
    pub id: String,
    /// 必须成功完成后本任务才可启动的任务 ID。
    pub dependencies: Vec<String>,
    /// 执行期间独占的资源 ID，例如同一构建输出目录。
    pub resources: Vec<String>,
}
