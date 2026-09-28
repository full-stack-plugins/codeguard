use serde::Deserialize;
/// 发行范围及原始清单/锁摘要，不复用白名单批准协议。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedDistributionPayload {
    pub schema_version: String,
    pub publisher_id: String,
    pub channel: String,
    pub platform: String,
    pub release_sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub manifest_sha256: String,
    pub lock_sha256: String,
}
