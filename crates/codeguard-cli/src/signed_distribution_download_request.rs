use crate::distribution_source::{DistributionTrustKey, DistributionVerificationContext};
use std::path::Path;
/// 独立宿主明确调用的签名下载输入；不从项目推导网络许可。
pub struct SignedDistributionDownloadRequest<'a> {
    /// 原始发行签名封装。
    pub envelope_bytes: &'a [u8],
    /// 被签原清单字节。
    pub manifest_bytes: &'a [u8],
    /// 被签原工具锁字节。
    pub lock_bytes: &'a [u8],
    /// 精确选择的工具 ID。
    pub tool_id: &'a str,
    /// 已存在受管缓存根。
    pub cache_root: &'a Path,
    /// 宿主固定发行信任密钥及撤销状态。
    pub trust: &'a DistributionTrustKey,
    /// 宿主固定范围、时钟和防回滚状态。
    pub context: &'a DistributionVerificationContext<'a>,
    /// 宿主允许的精确初始/重定向主机:端口，不允许通配或项目自批。
    pub network_authorities: &'a [&'a str],
}
