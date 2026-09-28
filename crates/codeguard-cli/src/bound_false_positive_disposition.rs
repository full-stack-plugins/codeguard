//! 验签和精确候选绑定后的只读处置预览，仍没有独立批准权威。

use crate::{ApprovalVerificationContext, VerifiedApprovalSnapshot};
use codeguard_core::AllowlistDisposition;

/// 固定候选/快照与宿主上下文的绑定结果；不能从项目 JSON 反序列化或自行构造。
#[derive(Debug)]
pub struct BoundFalsePositiveDisposition {
    preview: AllowlistDisposition,
    candidate_sha256: String,
    snapshot_sha256: String,
    workspace_id: String,
    baseline_commit: String,
    revision_sequence: u64,
    signing_key_id: String,
}

impl BoundFalsePositiveDisposition {
    pub(crate) fn new(
        preview: AllowlistDisposition,
        candidate_sha256: String,
        verified: &VerifiedApprovalSnapshot,
        context: &ApprovalVerificationContext<'_>,
    ) -> Self {
        Self {
            preview,
            candidate_sha256,
            snapshot_sha256: verified.snapshot_sha256().into(),
            workspace_id: context.workspace_id.into(),
            baseline_commit: context.baseline_commit.into(),
            revision_sequence: verified.revision_sequence(),
            signing_key_id: verified.signing_key_id().into(),
        }
    }
    /// 返回不具批准效果的只读字段；来源核验标记始终为 false。
    #[must_use]
    pub fn preview(&self) -> &AllowlistDisposition {
        &self.preview
    }
    /// 返回绑定的候选原始字节摘要，不代表候选来源可信。
    #[must_use]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }
    /// 返回已验签快照摘要，供宿主关联受保护来源。
    #[must_use]
    pub fn snapshot_sha256(&self) -> &str {
        &self.snapshot_sha256
    }
    /// 返回已核对的工作区范围。
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    /// 返回已核对的代码基线；替代链还须由 Git 联合入口核验关系。
    #[must_use]
    pub fn baseline_commit(&self) -> &str {
        &self.baseline_commit
    }
    /// 返回当前签名修订序号，宿主仍须固定最低值来源。
    #[must_use]
    pub fn revision_sequence(&self) -> u64 {
        self.revision_sequence
    }
    /// 返回验签公钥 ID，不能代替公钥来源认证。
    #[must_use]
    pub fn signing_key_id(&self) -> &str {
        &self.signing_key_id
    }
}
