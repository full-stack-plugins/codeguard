/// 签名与原始清单/锁的只读绑定；不证明宿主输入来源或安装授权。
#[derive(Debug)]
pub struct BoundDistributionSource {
    pub(crate) manifest_sha256: String,
    pub(crate) lock_sha256: String,
    pub(crate) platform: String,
    pub(crate) release_sequence: u64,
    pub(crate) expires_at: u64,
}
impl BoundDistributionSource {
    /// 返回精确原始清单摘要；不含下载 URL。
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    /// 返回精确原始工具锁摘要。
    pub fn lock_sha256(&self) -> &str {
        &self.lock_sha256
    }
    /// 返回签名绑定平台。
    pub fn platform(&self) -> &str {
        &self.platform
    }
    /// 返回经最低序号核对的发行序号。
    pub fn release_sequence(&self) -> u64 {
        self.release_sequence
    }
    /// 返回签名期限右边界，最终下载/发布时必须重新核对。
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}
