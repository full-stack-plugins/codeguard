/// 冻结的 HTTPS 包声明；来源授权须由调用方在网络请求前独立核验。
pub struct PackageDownloadRequest<'a> {
    /// 初始完整 HTTPS URL；不允许凭据、查询参数或片段。
    pub url: &'a str,
    /// 精确包字节数，最大 128 MiB。
    pub expected_size: u64,
    /// 同一来源声明中的非零包 SHA-256。
    pub expected_sha256: [u8; 32],
    /// 允许的跨源重定向 authority（规范主机:端口）；不从环境推导。
    pub redirect_authorities: &'a [&'a str],
}
