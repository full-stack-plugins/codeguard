use crate::distribution_source::{DistributionTrustKey, DistributionVerificationContext};
use std::path::Path;
/// 已明确调用发布的冻结输入；宿主信任来源真实性仍由调用方负责。
pub struct DistributionPublicationRequest<'a> {
    /// 原始签名封装字节。
    pub envelope_bytes: &'a [u8],
    /// 原始发行清单字节。
    pub manifest_bytes: &'a [u8],
    /// 原始工具锁字节，发布不重写它。
    pub lock_bytes: &'a [u8],
    /// 精确选择的工具 ID。
    pub tool_id: &'a str,
    /// 已取得的不可变发行包，本入口不联网。
    pub package: &'a [u8],
    /// 已存在的绝对受管缓存根。
    pub cache_root: &'a Path,
    /// 独立宿主发行密钥及撤销状态。
    pub trust: &'a DistributionTrustKey,
    /// 独立宿主预固定的范围/序号/期限。
    pub context: &'a DistributionVerificationContext<'a>,
}
