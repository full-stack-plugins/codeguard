/// 受保护宿主固定的发行签名公钥；不从项目配置加载。
#[derive(Clone, Debug)]
pub struct DistributionTrustKey {
    /// 发行信任库中的精确密钥 ID。
    pub key_id: String,
    /// 此密钥可签发的发行者身份。
    pub publisher_id: String,
    /// Ed25519 原始公钥。
    pub public_key: [u8; 32],
    /// 有效期左边界，Unix 秒。
    pub valid_from: u64,
    /// 有效期右边界，不包含。
    pub valid_until: u64,
    /// 宿主撤销状态；真实性由隔离宿主保证。
    pub revoked: bool,
}
