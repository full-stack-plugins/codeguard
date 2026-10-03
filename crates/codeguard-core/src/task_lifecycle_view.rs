use crate::TaskLifecycleState;
/// 有界事件链重放结果，不授予全项目 gate 权威。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskLifecycleView {
    /// 重建的状态。
    pub state: TaskLifecycleState,
    /// 唯一链尾；冲突时不选择有利节点。
    pub tip_event_id: Option<String>,
    /// 稳定诊断原因。
    pub reason: &'static str,
}
