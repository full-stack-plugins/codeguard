//! CLI 的 Unix 中断信号桥接；处理器只写原子标记，进程清理由执行循环负责。

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn mark_interrupted(_signal: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

/// 为当前 CLI 进程登记 Ctrl-C 取消；调用方须在执行原生任务前登记一次。
pub fn install_sigint_cancellation() -> io::Result<()> {
    INTERRUPTED.store(false, Ordering::Relaxed);
    // SAFETY: 处理器是具有 C ABI 的静态函数，只修改无锁原子布尔值。
    let previous = unsafe {
        libc::signal(
            libc::SIGINT,
            mark_interrupted as *const () as libc::sighandler_t,
        )
    };
    if previous == libc::SIG_ERR {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// 查询 CLI 收到的中断标记；未登记处理器的嵌入式调用保持 false。
#[must_use]
pub(crate) fn sigint_cancelled() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}
