//! 域分隔签名核验；不能将调用者提供的公钥来源自动升级为可信。

use codeguard_runtime::verify_approval_signature;
use sha2::{Digest, Sha256};

use crate::approval_trust_key::ApprovalTrustKey;
use crate::approval_verification_context::ApprovalVerificationContext;
use crate::signed_approval_envelope::SignedApprovalEnvelope;
use crate::signed_approval_payload::SignedApprovalPayload;
use crate::verified_approval_snapshot::VerifiedApprovalSnapshot;

/// 在预固定宿主公钥和上下文下核验原始签名及快照；失败返回受限原因码。
///
/// 参数 `snapshot_bytes` 是本轮使用的原始批准快照。返回值不证明快照协议有效、
/// 公钥来源可信或原生发现完整；须另经宿主来源验证和严格快照/候选绑定。
pub fn verify_signed_approval(
    envelope_bytes: &[u8],
    snapshot_bytes: &[u8],
    trust: &ApprovalTrustKey,
    context: &ApprovalVerificationContext<'_>,
) -> Result<VerifiedApprovalSnapshot, &'static str> {
    if envelope_bytes.len() > 16 * 1024 || snapshot_bytes.len() > 128 * 1024 {
        return Err("approval_input_limit_exceeded");
    }
    let now = context.now_unix.ok_or("approval_clock_unavailable")?;
    if !token(context.workspace_id)
        || !token(context.policy_revision)
        || !commit(context.baseline_commit)
        || context.minimum_sequence == 0
        || context.max_lifetime_seconds == 0
    {
        return Err("approval_context_invalid");
    }
    if !token(&trust.key_id) || trust.valid_from == 0 || trust.valid_until <= trust.valid_from {
        return Err("approval_trust_key_invalid");
    }
    if trust.revoked {
        return Err("approval_key_revoked");
    }
    if now < trust.valid_from || now >= trust.valid_until {
        return Err("approval_key_not_current");
    }
    let envelope: SignedApprovalEnvelope =
        serde_json::from_slice(envelope_bytes).map_err(|_| "approval_envelope_invalid")?;
    if envelope.schema_version != "1.0"
        || envelope.key_id != trust.key_id
        || envelope.approval_json.len() > 8 * 1024
    {
        return Err("approval_envelope_invalid");
    }
    let signature =
        decode_signature(&envelope.signature_hex).ok_or("approval_signature_invalid")?;
    // 精确签原文，绑定密钥 ID 和协议域；JSON 格式变动也必须重新签发。
    let mut message = b"codeguard.approval.v1\0".to_vec();
    message.extend_from_slice(envelope.key_id.as_bytes());
    message.push(0);
    message.extend_from_slice(envelope.approval_json.as_bytes());
    if !verify_approval_signature(&trust.public_key, &message, &signature) {
        return Err("approval_signature_invalid");
    }
    let payload: SignedApprovalPayload =
        serde_json::from_str(&envelope.approval_json).map_err(|_| "approval_payload_invalid")?;
    if payload.schema_version != "1.0"
        || payload.workspace_id != context.workspace_id
        || payload.policy_revision != context.policy_revision
        || payload.baseline_commit != context.baseline_commit
    {
        return Err("approval_scope_mismatch");
    }
    if payload.revision_sequence < context.minimum_sequence {
        return Err("approval_revision_rollback");
    }
    if payload.issued_at < trust.valid_from
        || payload.issued_at > now
        || now >= payload.expires_at
        || payload.expires_at > trust.valid_until
        || payload
            .expires_at
            .checked_sub(payload.issued_at)
            .is_none_or(|lifetime| lifetime == 0 || lifetime > context.max_lifetime_seconds)
    {
        return Err("approval_lifetime_invalid");
    }
    let snapshot_sha = format!("{:x}", Sha256::digest(snapshot_bytes));
    if payload.snapshot_sha256 != snapshot_sha {
        return Err("approval_snapshot_mismatch");
    }
    Ok(VerifiedApprovalSnapshot::new(
        snapshot_sha,
        payload.policy_revision,
        payload.revision_sequence,
        envelope.key_id,
        payload.issued_at,
        payload.expires_at,
    ))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
fn commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value.bytes().any(|byte| byte != b'0')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn decode_signature(value: &str) -> Option<[u8; 64]> {
    if value.len() != 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return None;
    }
    let mut bytes = [0; 64];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            (pair[0] as char).to_digit(16)? as u8 * 16 + (pair[1] as char).to_digit(16)? as u8;
    }
    Some(bytes)
}
