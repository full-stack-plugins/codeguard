use serde::{Deserialize, Serialize};

use crate::syntax_worker_recovery::SyntaxWorkerRecovery;

/// 私有工作进程的固定版本观察，父进程必须重新验证身份与位置。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyntaxWorkerEnvelope {
    /// 私有协议版本。
    pub schema_version: String,
    /// 固定类型。
    pub report_type: String,
    /// 语种 ID。
    pub language: String,
    /// 固定 grammar 字节摘要。
    pub grammar_sha256: String,
    /// 固定 grammar ABI。
    pub grammar_abi_version: u32,
    /// 源码原始字节摘要。
    pub source_sha256: String,
    /// 恢复节点或遍历预算是否截断。
    pub truncated: bool,
    /// ERROR/MISSING 恢复锚点。
    pub recoveries: Vec<SyntaxWorkerRecovery>,
}
