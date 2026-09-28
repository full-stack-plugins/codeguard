use codeguard_runtime::UnpackedArchive;
/// 与同一清单/锁关联的完整安装布局，只读且不授予来源或执行批准。
#[derive(Debug)]
pub struct DistributionLayout {
    pub(crate) tree: UnpackedArchive,
    pub(crate) install_tree_sha256: String,
    pub(crate) origin_ref: String,
    pub(crate) bundle_root: Option<String>,
    pub(crate) lock_sha256: String,
    pub(crate) manifest_sha256: String,
}
impl DistributionLayout {
    /// 返回完整原包布局，包含入口及 bundle 外成员；不转交可写引用。
    #[must_use]
    pub fn tree(&self) -> &UnpackedArchive {
        &self.tree
    }
    /// 返回本次匹配的完整安装树摘要，供发布层重新核验。
    #[must_use]
    pub fn install_tree_sha256(&self) -> &str {
        &self.install_tree_sha256
    }
    /// 返回与工具锁精确一致的缓存相对入口定位。
    #[must_use]
    pub fn origin_ref(&self) -> &str {
        &self.origin_ref
    }
    /// 返回与工具锁一致的可选 bundle 相对根；未声明时为 None。
    #[must_use]
    pub fn bundle_root(&self) -> Option<&str> {
        self.bundle_root.as_deref()
    }
    /// 返回原工具锁字节摘要，不能作为锁批准来源。
    #[must_use]
    pub fn lock_sha256(&self) -> &str {
        &self.lock_sha256
    }
    /// 返回本轮清单原字节摘要，不能作为清单批准来源。
    #[must_use]
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
}
