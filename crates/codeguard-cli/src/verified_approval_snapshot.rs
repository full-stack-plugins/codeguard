//! 签名及上下文核对结果，不包含门禁或原生检查完整性声明。

/// 已在给定宿主公钥和上下文下通过密码学核验的快照身份。
#[derive(Debug)]
pub struct VerifiedApprovalSnapshot {
    snapshot_sha256: String,
    policy_revision: String,
    revision_sequence: u64,
    signing_key_id: String,
    issued_at: u64,
    expires_at: u64,
}

impl VerifiedApprovalSnapshot {
    pub(crate) fn new(
        sha: String,
        revision: String,
        sequence: u64,
        key_id: String,
        issued_at: u64,
        expires_at: u64,
    ) -> Self {
        Self {
            snapshot_sha256: sha,
            policy_revision: revision,
            revision_sequence: sequence,
            signing_key_id: key_id,
            issued_at,
            expires_at,
        }
    }

    /// 返回签名绑定的原始快照 SHA-256；供后续严格快照解析核对。
    #[must_use]
    pub fn snapshot_sha256(&self) -> &str {
        &self.snapshot_sha256
    }
    /// 返回已核对的策略修订，不代表当前检查已完成。
    #[must_use]
    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }
    /// 返回已核对的防回滚序号，宿主仍须持久固定最低值。
    #[must_use]
    pub fn revision_sequence(&self) -> u64 {
        self.revision_sequence
    }
    /// 返回签名公钥 ID，其可信来源由宿主负责。
    #[must_use]
    pub fn signing_key_id(&self) -> &str {
        &self.signing_key_id
    }
    /// 返回签名绑定的签发时刻，用于核对修订链顺序。
    #[must_use]
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    /// 返回已验签批准的到期上限；宿主不得用更长的候选期限代替此边界。
    #[must_use]
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}
