//! 受保护宿主提供的批准签名公钥；项目文件不能作为本对象的信任来源。

/// 宿主预先固定的公钥和有效期；调用方负责信任来源及撤销状态真实性。
#[derive(Clone, Debug)]
pub struct ApprovalTrustKey {
    /// 受保护信任库中的精确密钥 ID。
    pub key_id: String,
    /// Ed25519 原始 32 字节公钥。
    pub public_key: [u8; 32],
    /// 密钥生效时间，Unix 秒。
    pub valid_from: u64,
    /// 密钥失效时间，Unix 秒，右边界不包含。
    pub valid_until: u64,
    /// 受保护信任库是否已撤销此密钥。
    pub revoked: bool,
}
