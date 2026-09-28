//! 原生进程的终止原因，不将工具退出码解释为规则结果。

/// 执行层真实终止原因；仅 Exited 含原生退出码。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Termination {
    /// 进程正常退出；规则语义仍由具体 adapter 解释。
    Exited(i32),
    /// 无可用整数退出码的异常终止。
    Signaled,
    /// 调用契约无效。
    InvalidSpec,
    /// 开始前共同预算已经耗尽。
    DeadlineBeforeStart,
    /// 用户或上层请求取消。
    Cancelled,
    /// 共同截止时间到达。
    TimedOut,
    /// stdout/stderr 合计超出预算。
    OutputLimit,
    /// 进程无法启动。
    SpawnFailure,
    /// 管道读取失败。
    ReadFailure,
    /// 显式 stdin 未能写完。
    WriteFailure,
    /// 受控进程组无法保证清理。
    CleanupFailure,
    /// 当前平台尚无可验证的整组回收能力。
    UnsupportedPlatform,
}
