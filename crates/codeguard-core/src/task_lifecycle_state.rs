use serde::{Deserialize, Serialize};
/// 追加事件链重建的任务状态；未经验证的关闭记录不是解决事实。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecycleState {
    /// 存在待处理原问题。
    Open,
    /// 仅该问题已通过真实复检解决。
    Resolved,
    /// 关闭记录缺独立验证或本地证据。
    VerificationRequired,
    /// 分叉、循环、缺父节点或归属冲突须核对。
    ReconciliationRequired,
}
