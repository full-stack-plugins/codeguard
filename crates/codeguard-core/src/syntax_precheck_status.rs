use serde::{Deserialize, Serialize};

/// 语法初检的总体状态，与原生检查和交付结论分离。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxPrecheckStatus {
    /// 没有选定文件，初检没有执行。
    NotRun,
    /// 非空选定范围全部由已验收 grammar 检查，且无恢复节点。
    Clean,
    /// 完整初检观察到 ERROR/MISSING，仍需原生确认。
    SuspectedIssue,
    /// 有未完成或范围混合缺口；已观察疑似节点仍保留。
    Incomplete,
    /// 全部选定文件均不受支持。
    Unsupported,
}
