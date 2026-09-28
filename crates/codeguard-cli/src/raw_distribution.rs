use std::fmt;
/// 同一原锁/清单绑定的冻结 raw 字节视图；不复制内容或授予批准。
pub struct RawDistribution<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) binary_sha256: [u8; 32],
    pub(crate) origin_ref: String,
    pub(crate) lock_sha256: String,
    pub(crate) manifest_sha256: String,
}
impl RawDistribution<'_> {
    /// 返回本轮已核对的不可变原包字节；借用存续期间不能修改输入。
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
    /// 返回与包/入口共同绑定的二进制摘要，发布时仍须重验。
    #[must_use]
    pub fn binary_sha256(&self) -> [u8; 32] {
        self.binary_sha256
    }
    /// 返回与原锁精确一致的缓存相对入口。
    #[must_use]
    pub fn origin_ref(&self) -> &str {
        &self.origin_ref
    }
    /// 返回本轮锁原字节摘要，不代表可信锁来源。
    #[must_use]
    pub fn lock_sha256(&self) -> &str {
        &self.lock_sha256
    }
    /// 返回本轮清单原字节摘要，不代表发行批准。
    #[must_use]
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
}
impl fmt::Debug for RawDistribution<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 诊断不输出原包内容，避免意外转储制品字节。
        f.debug_struct("RawDistribution")
            .field("byte_len", &self.bytes.len())
            .field("binary_sha256", &self.binary_sha256)
            .finish_non_exhaustive()
    }
}
