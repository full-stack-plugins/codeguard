/// 归档展开的冻结输入；结构与摘要匹配不授予发行来源批准。
pub struct ArchiveUnpackRequest<'a> {
    /// 支持 zip 或 tar_gz。
    pub format: &'a str,
    /// 必须存在且摘要精确匹配的相对入口。
    pub entrypoint: &'a str,
    /// 传输包的预期 SHA-256。
    pub package_sha256: [u8; 32],
    /// 原生工具入口的预期 SHA-256。
    pub entrypoint_sha256: [u8; 32],
    /// 全部普通文件展开字节上限，不超过 512 MiB。
    pub max_unpacked_bytes: u64,
}
