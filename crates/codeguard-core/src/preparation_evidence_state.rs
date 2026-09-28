//! 前置条件观察状态；工具未探测不能当作确认缺失。
use serde::{Deserialize, Serialize};
/// 单项准备观察的语义，不描述源码 finding 或原生检查完成度。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationEvidenceState {
    /// 原生或受保护配置证据确认满足。
    Satisfied,
    /// 当前有效证据确认缺失。
    Missing,
    /// 当前有效证据确认不兼容。
    Incompatible,
    /// 当前有效证据确认冲突。
    Conflict,
    /// 尚未探测。
    Unprobed,
    /// 已观察但未解析或诊断不完整。
    Unresolved,
}
