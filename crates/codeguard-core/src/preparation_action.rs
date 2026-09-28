//! 准备任务动作，不是可自动执行的命令或源码修复。
use serde::{Deserialize, Serialize};

/// 仅针对本次可信前置诊断的下一步类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationAction {
    /// 当前证据确认缺失，应恢复对应前置。
    RestoreMissingPrerequisite,
    /// 当前证据确认不兼容，应核对要求与本机工具。
    ResolveCompatibility,
    /// 当前证据确认冲突，应保留输入并提出具体决策。
    ResolveConflict,
    /// 观察未确认、失效或不匹配，应重新解析/探测。
    ReverifyPrerequisite,
}
