use codeguard_adapters::EslintCommand;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

/// 显式 ESLint 局部执行请求；输入摘要由独立调用上下文冻结，不自证批准。
pub struct EslintProbeRequest {
    /// 显式原生字面命令。
    pub command: EslintCommand,
    /// 本轮物理绝对工作目录；模块/插件上下文归属须由上层核验。
    pub cwd: PathBuf,
    /// 预建私有日志目录。
    pub evidence_dir: PathBuf,
    /// 有界安全运行 ID；报告文件必须同名 JSON。
    pub run_id: String,
    /// 独立预期的具体稳定 ESLint 10 版本。
    pub expected_version: String,
    /// Node、JS 入口、原配置与所有显式源码的精确摘要集合。
    pub expected_sha256: BTreeMap<PathBuf, [u8; 32]>,
    /// 版本、扫描、留证与读取共用绝对截止时间。
    pub deadline: Instant,
}
