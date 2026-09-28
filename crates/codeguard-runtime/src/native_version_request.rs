//! 原生版本探测输入，只允许固定版本调用。
use crate::ProcessSpec;
use std::path::PathBuf;

/// 宿主/适配器冻结的工具身份和版本契约，不是批准协议。
#[derive(Clone, Debug)]
pub struct NativeVersionRequest {
    /// 受控进程规格；版本探测不能含源码参数或 stdin。
    pub process: ProcessSpec,
    /// 本次应保持不变的工具原始字节 SHA-256。
    pub expected_tool_sha256: [u8; 32],
    /// 适配器声明的精确原生版本输出，含换行。
    pub expected_stdout: Vec<u8>,
    /// 预先建立的私有本轮证据目录。
    pub evidence_root: PathBuf,
    /// 不可复用的本轮日志文件名。
    pub log_name: String,
}
