use serde::Serialize;

use crate::HookTriggerAction;

/// 事件路由的只读候选计划；没有执行证据，不可签发交付通过。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HookTriggerPlan {
    /// 本事件请求的检查阶段。
    pub action: HookTriggerAction,
    /// 编辑快检的目标路径；交付阶段必须忽略宿主猜测的路径集合。
    pub target_paths: Vec<String>,
    /// 原检查器复检目标。
    pub task_id: Option<String>,
    /// 是否须从本轮 Git 获取真实提交或推送快照。
    pub requires_git_snapshot: bool,
    /// 局部完整结果是否可作为软反馈复用候选；复用仍需核验全部身份。
    pub soft_result_reuse_candidate: bool,
    /// 宿主对阻断能力的声明，仅供后续真实宿主核验。
    pub host_blocking_claimed: bool,
    /// 规划从不证明检查已完成或允许交付。
    pub may_claim_delivery: bool,
}
