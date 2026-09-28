//! 按签名原文严格解析的批准载荷。

use serde::Deserialize;

/// 签名载荷的工作区、基线和原始快照字节绑定。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedApprovalPayload {
    pub schema_version: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub baseline_commit: String,
    pub revision_sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub snapshot_sha256: String,
}
