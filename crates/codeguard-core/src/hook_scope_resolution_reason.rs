use serde::Serialize;

/// 编辑事件不能安全逐文件快检时，宿主需要如何重定范围的原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookScopeResolutionReason {
    /// 确认写入成功，但没有可靠目标路径。
    ChangedPathsMissing,
    /// 宿主不能确认写入结果，需重新观察工作区。
    WriteOutcomeUnknown,
    /// 路径数量或长度超出快检预算，需改用有界批量范围。
    FastScopeBudgetExceeded,
}
