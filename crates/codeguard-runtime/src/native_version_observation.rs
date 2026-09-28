//! 原生版本的本地观察，不授予工具锁或质量通过。
use crate::Termination;

/// 保留执行失败类别；原始输出仅在私有证据目录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeVersionObservation {
    /// 仅精确版本、工具字节和本轮证据均完整时为真。
    pub complete: bool,
    /// 稳定诊断码，不含原生文本或环境变量。
    pub reason: Option<&'static str>,
    /// 保留原生终止类别；工具身份未确认时没有进程结果。
    pub termination: Option<Termination>,
}
