use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// 显式选择的 Checkstyle 单文件本地观察请求；摘要由独立调用上下文冻结，不是批准证明。
pub struct CheckstyleProbeRequest {
    /// 受控源码快照的物理绝对路径。
    pub source: PathBuf,
    /// 原配置快照的物理绝对路径。
    pub config: PathBuf,
    /// 显式 Java 入口；完整 JDK 运行时闭包仍由上层核验。
    pub java: PathBuf,
    /// 显式自包含 Checkstyle JAR。
    pub jar: PathBuf,
    /// 精确四个输入路径与 SHA-256；不自动填入或批准本地内容。
    pub expected_sha256: BTreeMap<PathBuf, [u8; 32]>,
    /// 调用方预建的私有报告目录。
    pub report_dir: PathBuf,
    /// 调用方预建的私有日志目录。
    pub evidence_dir: PathBuf,
    /// 单轮安全文件名前缀。
    pub run_id: String,
    /// 独立预期 Checkstyle 版本。
    pub expected_version: String,
    /// 本轮各阶段共享的绝对截止。
    pub deadline: Instant,
}
