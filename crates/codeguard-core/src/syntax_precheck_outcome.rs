use serde::{Deserialize, Serialize};

use crate::SyntaxPrecheckStatus;

/// 选定文件集的纯领域聚合；不能作为 lint 通过或项目门禁依据。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyntaxPrecheckOutcome {
    /// 总体初检状态。
    pub status: SyntaxPrecheckStatus,
    /// 选定范围的枚举与输入核对是否完成。
    pub scope_complete: bool,
    /// 本次初检是否收到取消请求。
    pub cancelled: bool,
    /// 选定源码文件数。
    pub selected_files: usize,
    /// 已解析文件数，含版本/方言尚未验收者。
    pub checked_files: usize,
    /// 解析或输入核对未完成的文件数。
    pub incomplete_files: usize,
    /// 不支持的文件数。
    pub unsupported_files: usize,
    /// 虽已解析但 grammar 版本/方言未获验收的文件数。
    pub unqualified_files: usize,
    /// 恢复节点或遍历预算截断的文件数。
    pub truncated_files: usize,
    /// 已观察 ERROR/MISSING 数；疑似观察不等于原生违规。
    pub suspected_recoveries: usize,
}
