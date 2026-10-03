use serde::{Deserialize, Serialize};
/// 解决归因；策略处置和目标删除不得统计为代码修复。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionCause {
    /// 当前源码真实修复。
    CodeFixed,
    /// 当前依赖或依赖锁真实修复。
    DependencyFixed,
    /// 原受阻检查已恢复并完成。
    EnvironmentRestored,
    /// 目标经批准和范围证明移除。
    TargetRemoved,
    /// 经批准策略修订处置。
    PolicyResolved,
}
