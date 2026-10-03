use crate::{TaskIdentity, TaskLifecycleKind};
use serde::{Deserialize, Serialize};
/// 单个不可变生命周期事件；父关系替代时间排序，文件自报状态不能授权。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskLifecycleEvent {
    /// 事件稳定身份，应用层须核对文件名和原字节。
    pub event_id: String,
    /// 首次观察无父节点；之后引用精确前一事件。
    pub parent_event_id: Option<String>,
    /// 精确任务归属。
    pub identity: TaskIdentity,
    /// 生命周期动作及证据引用。
    pub kind: TaskLifecycleKind,
}
