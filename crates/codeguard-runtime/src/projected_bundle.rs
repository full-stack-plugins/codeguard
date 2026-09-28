use crate::UnpackedArchive;
/// 已完成子树内容匹配的只读结果；不授予来源、安装或准备权限。
#[derive(Debug)]
pub struct ProjectedBundle {
    archive_root: String,
    tree_sha256: String,
    tree: UnpackedArchive,
    remainder: UnpackedArchive,
}
impl ProjectedBundle {
    /// 由匹配服务记录原包前缀、预期摘要、投影及未消费成员。
    pub(crate) fn new(
        archive_root: String,
        tree_sha256: String,
        tree: UnpackedArchive,
        remainder: UnpackedArchive,
    ) -> Self {
        Self {
            archive_root,
            tree_sha256,
            tree,
            remainder,
        }
    }
    /// 返回本次显式包内前缀；空字符串表示明确选择整棵树。
    #[must_use]
    pub fn archive_root(&self) -> &str {
        &self.archive_root
    }
    /// 返回已匹配的预期树摘要；摘要来源可信性仍须调用方独立核验。
    #[must_use]
    pub fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }
    /// 返回去掉包前缀后的只读完整树，包含空目录。
    #[must_use]
    pub fn tree(&self) -> &UnpackedArchive {
        &self.tree
    }
    /// 返回未被 bundle 消费的原包成员，保留原路径和内容。
    #[must_use]
    pub fn remainder(&self) -> &UnpackedArchive {
        &self.remainder
    }
    /// 消费结果并交出两个树的所有权；后续发布必须重新核验内容/权限。
    #[must_use]
    pub fn into_parts(self) -> (UnpackedArchive, UnpackedArchive) {
        (self.tree, self.remainder)
    }
}
