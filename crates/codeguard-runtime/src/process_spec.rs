//! 原生工具的字面调用和共同预算。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Instant;

/// 受控进程调用；截止时间由上层请求统一创建，重试不得重置。
#[derive(Clone, Debug)]
pub struct ProcessSpec {
    /// 已解析且验证过的绝对可执行文件路径。
    pub executable: PathBuf,
    /// 逐项字面参数，不经过 Shell。
    pub args: Vec<OsString>,
    /// 明确的工作目录。
    pub cwd: PathBuf,
    /// 完整白名单环境；继承环境会被清空。
    pub env: BTreeMap<OsString, OsString>,
    /// None 表示关闭 stdin；Some 为显式输入字节。
    pub stdin: Option<Vec<u8>>,
    /// 整个请求传递下来的单一绝对截止时间。
    pub deadline: Instant,
    /// stdout 与 stderr 合计最大保留字节数。
    pub output_limit_bytes: usize,
}
