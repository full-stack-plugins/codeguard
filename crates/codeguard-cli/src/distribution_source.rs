//! 发行签名与同一原始清单/锁的静态绑定，不提供签发或安装入口。
pub use crate::bound_distribution_source::BoundDistributionSource;
use crate::distribution_manifest::{bind_distribution_manifest, parse_distribution_manifest};
pub use crate::distribution_trust_key::DistributionTrustKey;
pub use crate::distribution_verification_context::DistributionVerificationContext;
use crate::signed_distribution_envelope::SignedDistributionEnvelope;
use crate::signed_distribution_payload::SignedDistributionPayload;
use codeguard_runtime::verify_approval_signature;
use sha2::{Digest, Sha256};
/// 核验域分隔发行签名及原始清单、锁、平台和时间范围。
/// 参数含原始封装/清单/锁及独立宿主信任和上下文；返回只读字节绑定。
/// 调用者提供的公钥、时钟及最低序号不自证可信，本函数不联网/安装/授予门禁权威。
pub fn verify_signed_distribution(
    envelope_bytes: &[u8],
    manifest_bytes: &[u8],
    lock_bytes: &[u8],
    trust: &DistributionTrustKey,
    context: &DistributionVerificationContext<'_>,
) -> Result<BoundDistributionSource, &'static str> {
    if envelope_bytes.len() > 16 * 1024
        || manifest_bytes.len() > 256 * 1024
        || lock_bytes.len() > 128 * 1024
    {
        return Err("distribution_signature_input_limit");
    }
    let now = context.now_unix.ok_or("distribution_clock_unavailable")?;
    if now == 0
        || !token(context.publisher_id)
        || !token(context.channel)
        || !token(context.platform)
        || context.minimum_sequence == 0
        || context.max_lifetime_seconds == 0
    {
        return Err("distribution_context_invalid");
    }
    if !token(&trust.key_id)
        || trust.publisher_id != context.publisher_id
        || trust.valid_from == 0
        || trust.valid_until <= trust.valid_from
        || trust.public_key == [0; 32]
    {
        return Err("distribution_trust_invalid");
    }
    if trust.revoked {
        return Err("distribution_key_revoked");
    }
    if now < trust.valid_from || now >= trust.valid_until {
        return Err("distribution_key_not_current");
    }
    let envelope: SignedDistributionEnvelope =
        serde_json::from_slice(envelope_bytes).map_err(|_| "distribution_envelope_invalid")?;
    if envelope.schema_version != "1.0"
        || envelope.key_id != trust.key_id
        || envelope.release_json.len() > 8 * 1024
    {
        return Err("distribution_envelope_invalid");
    }
    let signature =
        decode_signature(&envelope.signature_hex).ok_or("distribution_signature_invalid")?;
    let mut message = b"codeguard.distribution.v1\0".to_vec();
    message.extend_from_slice(envelope.key_id.as_bytes());
    message.push(0);
    message.extend_from_slice(envelope.release_json.as_bytes());
    if !verify_approval_signature(&trust.public_key, &message, &signature) {
        return Err("distribution_signature_invalid");
    }
    let payload: SignedDistributionPayload =
        serde_json::from_str(&envelope.release_json).map_err(|_| "distribution_payload_invalid")?;
    if payload.schema_version != "1.0"
        || payload.publisher_id != context.publisher_id
        || payload.channel != context.channel
        || payload.platform != context.platform
    {
        return Err("distribution_scope_mismatch");
    }
    if payload.release_sequence < context.minimum_sequence {
        return Err("distribution_revision_rollback");
    }
    if payload.issued_at < trust.valid_from
        || payload.issued_at > now
        || now >= payload.expires_at
        || payload.expires_at > trust.valid_until
        || payload
            .expires_at
            .checked_sub(payload.issued_at)
            .is_none_or(|v| v == 0 || v > context.max_lifetime_seconds)
    {
        return Err("distribution_lifetime_invalid");
    }
    let manifest_sha256 = format!("{:x}", Sha256::digest(manifest_bytes));
    let lock_sha256 = format!("{:x}", Sha256::digest(lock_bytes));
    if payload.manifest_sha256 != manifest_sha256 {
        return Err("distribution_manifest_digest_mismatch");
    }
    if payload.lock_sha256 != lock_sha256 {
        return Err("distribution_signed_lock_mismatch");
    }
    // 即使签名和摘要相符，也必须重新严格解析，并核对原锁中的每个制品。
    let manifest = parse_distribution_manifest(manifest_bytes)?;
    let artifacts = bind_distribution_manifest(&manifest, lock_bytes)?;
    if !artifacts.iter().any(|a| a.platform == context.platform) {
        return Err("distribution_platform_unbound");
    }
    Ok(BoundDistributionSource {
        manifest_sha256,
        lock_sha256,
        platform: payload.platform,
        release_sequence: payload.release_sequence,
        expires_at: payload.expires_at,
    })
}
fn token(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}
fn decode_signature(v: &str) -> Option<[u8; 64]> {
    if v.len() != 128
        || !v
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut result = [0; 64];
    for (i, pair) in v.as_bytes().chunks_exact(2).enumerate() {
        result[i] =
            (pair[0] as char).to_digit(16)? as u8 * 16 + (pair[1] as char).to_digit(16)? as u8;
    }
    Some(result)
}
