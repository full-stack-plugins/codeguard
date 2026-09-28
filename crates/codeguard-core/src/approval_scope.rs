//! 批准来源所限定的工作区与受保护代码基线。

use serde::{Deserialize, Serialize};

/// 由宿主独立冻结的批准范围；不是工作树路径或可变 Git 引用。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalScope {
    /// 受保护宿主分配的工作区标识。
    pub workspace_id: String,
    /// 当前批准上下文的完整非零 SHA-1 或 SHA-256 提交身份。
    pub baseline_commit: String,
}

impl ApprovalScope {
    pub(crate) fn valid(&self) -> bool {
        !self.workspace_id.is_empty()
            && self.workspace_id.len() <= 128
            && self.workspace_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
            && matches!(self.baseline_commit.len(), 40 | 64)
            && self
                .baseline_commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            && self.baseline_commit.bytes().any(|byte| byte != b'0')
    }
}
