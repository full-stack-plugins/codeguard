use codeguard_adapters::NpmAuditCommand;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
/// npm原生局部请求；配置、源码与工具身份由调用者冻结，不自证批准。
pub struct NpmAuditProbeRequest {
    /// 显式Node/npm及独立配置、缓存入口。
    pub command: NpmAuditCommand,
    /// 物理绝对项目目录。
    pub cwd: PathBuf,
    /// 预建私有报告与日志目录。
    pub evidence_dir: PathBuf,
    /// 安全且有界的本轮运行ID。
    pub run_id: String,
    /// 具体稳定npm11版本。
    pub expected_version: String,
    /// 显式审计源；None仅离线观察，不能确认库覆盖。
    pub registry: Option<String>,
    /// 工具入口、清单、锁、用户/全局配置及存在的项目npmrc精确字节摘要。
    pub expected_sha256: BTreeMap<PathBuf, [u8; 32]>,
    /// 版本、审计、读取与输入核对共享截止时间。
    pub deadline: Instant,
}
