//! 发行清单中的精确制品声明；不会自动取得安装权威。
use serde::Deserialize;
/// 单个工具/平台的包与入口身份，供批准计划绑定和后续有界下载使用。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionArtifact {
    /// 工具规范 ID。
    pub tool_id: String,
    /// 固定原生版本。
    pub version: String,
    /// 目标平台。
    pub platform: String,
    /// 工具入口字节摘要。
    pub binary_sha256: String,
    /// 锁中来源引用的字节摘要。
    pub origin_ref_sha256: String,
    /// HTTPS 下载地址；读取不发起网络请求。
    pub download_url: String,
    /// 整个原始下载包摘要。
    pub package_sha256: String,
    /// 精确压缩/下载字节数。
    pub package_size_bytes: u64,
    /// raw、tar_gz 或 zip。
    pub format: String,
    /// 压缩包内相对工具入口；raw 必须为空。
    pub entrypoint: Option<String>,
    /// 展开字节的硬上限；raw 与原字节数一致。
    pub unpacked_size_limit_bytes: u64,
    /// 委托工具包树身份；没有目录包时为空。
    pub bundle_tree_sha256: Option<String>,
    /// 1.1 显式包内 bundle 目录；空字符串选整树，None 表示未声明。
    pub bundle_archive_root: Option<String>,
    /// 1.2 完整安装树摘要，包含入口与未归入 bundle 的普通成员。
    pub install_tree_sha256: Option<String>,
}
