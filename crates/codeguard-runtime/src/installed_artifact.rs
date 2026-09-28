//! 受管工具字节发布收据，不承载批准或质量状态。
/// 缓存中已核对的制品定位；仅描述本次发布或复用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledArtifact {
    /// 缓存根下的内容寻址普通文件名，不含宿主绝对路径。
    pub relative_path: String,
    /// 是否本次新发布；false 表示已有制品重新核对成功。
    pub newly_published: bool,
    /// 经核对的原始字节数，不是安装包展开大小。
    pub byte_len: u64,
}
