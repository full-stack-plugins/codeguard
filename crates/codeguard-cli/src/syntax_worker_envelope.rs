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
    /// 1.4/1.5/1.6明确记录不可定位的解析树错误；更旧版本不得携带该字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parser_error_location_unavailable: Option<bool>,
    /// ERROR/MISSING 恢复锚点。
    pub recoveries: Vec<SyntaxWorkerRecovery>,
    /// Python1.1、Go1.2、CFQuery1.3及显式JavaScript1.5/Erlang1.6结构观察；1.0中不存在。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structural_observations: Vec<crate::syntax_worker_structure::SyntaxWorkerStructure>,
}
