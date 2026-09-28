//! 签名封装；载荷使用原始 UTF-8 文本，不重新序列化后验签。

use serde::Deserialize;

/// 域分隔 Ed25519 签名及被签原文。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedApprovalEnvelope {
    pub schema_version: String,
    pub key_id: String,
    pub approval_json: String,
    pub signature_hex: String,
}
