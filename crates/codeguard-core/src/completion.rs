//! 单项义务的执行完整性。

use serde::{Deserialize, Serialize};

/// 检查器是否证明了选定义务，与是否发现违规正交。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completion {
    /// 目标、规则和报告均完成并通过身份核对。
    Complete,
    /// 工具、配置、证据或覆盖存在缺口。
    Incomplete,
    /// 完整发现后按结构性理由证明不适用。
    NotApplicable,
}
