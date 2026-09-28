use crate::{HookEvent, HookWriteOutcome};
use serde::Deserialize;

/// 宿主事件的未核验输入；规划器不能把它当成代码或 Git 事实。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HookTriggerInput {
    /// 事件阶段。
    pub event: HookEvent,
    /// 编辑事件携带的工作区相对路径；交付事件必须重新获取 Git 范围。
    pub changed_paths: Vec<String>,
    /// 修复复检事件的稳定任务 ID。
    pub task_id: Option<String>,
    /// 宿主对编辑工具写入结果的观察；不证明实际字节身份。
    pub write_outcome: HookWriteOutcome,
    /// 宿主声明是否支持可靠阻断；此输入尚未经真实宿主协议验收。
    pub host_claims_blocking: bool,
}
