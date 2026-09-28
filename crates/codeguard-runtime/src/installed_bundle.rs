/// 完整工具目录发布的相对收据；不表示发行批准或工具可运行。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledBundle {
    /// 缓存根中的内容寻址目录名。
    pub relative_path: String,
    /// 本轮新发布为 true；已存在目录重新核对成功为 false。
    pub newly_published: bool,
    /// 核对的全部普通文件字节数。
    pub byte_len: u64,
}
