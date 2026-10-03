use serde::{Deserialize, Serialize};
/// 持久任务的精确归属；不能用相似规则或相邻文件替代。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskIdentity {
    /// 工作区身份。
    pub workspace_id: String,
    /// 稳定任务身份。
    pub task_id: String,
    /// 原检查器身份。
    pub checker_id: String,
    /// 仓库相对的精确目标范围。
    pub scope: String,
}
