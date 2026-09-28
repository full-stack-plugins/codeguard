//! 新 CLI 的请求级退出语义。

use serde::{Deserialize, Serialize};

/// 请求结论；用法错误由执行前参数解析产生，不在检查结果里伪造。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// 已选义务完整且无阻断发现。
    Passed,
    /// 完整交付检查仅因经批准的精确误报例外而可通过；必须显示批准引用。
    PassedWithExceptions,
    /// 已选义务完整，但存在阻断发现。
    Violations,
    /// 至少一个必需义务未完成。
    Incomplete,
    /// 完整观察后所有已选义务均不适用；不等于交付 allow。
    NotApplicable,
    /// Codeguard 内部无法建立可信结果。
    InternalError,
    /// 用户取消；保留此前取得的有效发现。
    Cancelled,
}

impl Verdict {
    /// 返回版本化新 CLI 的退出码。
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Passed | Self::PassedWithExceptions | Self::NotApplicable => 0,
            Self::Violations => 1,
            Self::Incomplete => 3,
            Self::InternalError => 4,
            Self::Cancelled => 130,
        }
    }
}
