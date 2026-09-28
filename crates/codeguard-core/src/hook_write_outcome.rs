use serde::{Deserialize, Serialize};

/// 编辑工具的写入结果；失败不触发源码检查，未知结果需重新确定范围。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookWriteOutcome {
    /// 宿主确认写入成功；仍须核对真实源码身份。
    Confirmed,
    /// 宿主确认未写入。
    Failed,
    /// 宿主无法确认是否写入。
    Unknown,
}
