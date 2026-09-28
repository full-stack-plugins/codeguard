//! 工作区内稳定准备任务投影；持久化与原生复检由宿主实现。
use crate::{PreparationAction, PreparationEvidenceState};
use serde::{Deserialize, Serialize};

/// 准备任务的可审阅描述，不具备批准、安装或门禁权限。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationTask {
    /// 工作区内逻辑键，不含绑定/时间；不得直接用作文件路径。
    pub task_key: String,
    /// 所属批准前置要求标识。
    pub prerequisite_id: String,
    /// 本轮策略/工具/目标配置绑定，变化时更新而非创建重复任务。
    pub binding_sha256: String,
    /// 下一步类别，不能解释为执行授权。
    pub action: PreparationAction,
    /// 仅已确认的当前阻塞状态；未知任务不携带历史诊断。
    pub confirmed_state: Option<PreparationEvidenceState>,
    /// 固定模板步骤，不拼接原生输出或项目命令。
    pub instruction: String,
    /// 关闭需重新获得满足证据，勾选任务不能替代复检。
    pub close_condition: String,
    /// 规划始终不授权自动执行或安装。
    pub automatic_execution_authorized: bool,
}
