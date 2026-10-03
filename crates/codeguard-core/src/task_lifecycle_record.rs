use crate::TaskLifecycleEvent;
use serde::{Deserialize, Serialize};
/// 脱敏生命周期记录；摘要用于内容关联，不授予可编辑项目历史关闭权威。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskLifecycleRecord {
    /// 持久记录协议版本。
    pub schema_version: String,
    /// 限定的追加事件记录类型。
    pub report_type: String,
    /// 任务动作与明确父事件身份。
    pub event: TaskLifecycleEvent,
    /// 不可改写的首次任务观察字节摘要。
    pub original_report_sha256: String,
    /// 本地脱敏对照证据摘要；根观察无此引用。
    pub evidence_sha256: Option<String>,
    /// 本次验签策略字节摘要；不保存自批准布尔值。
    pub policy_sha256: Option<String>,
}
