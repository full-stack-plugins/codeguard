//! 历史批准的签名、候选和宿主记录，不能由项目自造历史上下文。
use crate::{ApprovalTrustKey, ApprovalVerificationContext};

/// 一跳历史批准；审核时刻、基线和最低序号须来自受保护历史记录。
#[derive(Clone, Copy, Debug)]
pub struct SignedPriorApprovalInput<'a> {
    /// 前序候选原始字节。
    pub decision_bytes: &'a [u8],
    /// 前序批准快照原始字节。
    pub snapshot_bytes: &'a [u8],
    /// 前序快照签名封装。
    pub envelope_bytes: &'a [u8],
    /// 宿主公钥和最新撤销状态。
    pub trust: &'a ApprovalTrustKey,
    /// 已固定历史审核上下文，now_unix 是受保护记录的审核时刻。
    pub context: ApprovalVerificationContext<'a>,
}
