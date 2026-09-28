/// 签名/内容匹配后的工具制品发布收据，不表示运行时齐备或可执行。
#[derive(Debug)]
pub struct PublishedDistributionArtifact {
    pub(crate) origin_ref: String,
    pub(crate) newly_published: bool,
}
impl PublishedDistributionArtifact {
    /// 返回与原工具锁精确一致的相对入口，隐藏缓存绝对路径。
    pub fn origin_ref(&self) -> &str {
        &self.origin_ref
    }
    /// 返回本轮是否新发布；复用前也会重新核验签名与内容。
    pub fn newly_published(&self) -> bool {
        self.newly_published
    }
}
