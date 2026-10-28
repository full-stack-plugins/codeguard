//! CLI 的平台中断信号桥接；处理器只写原子标记，进程清理由执行循环负责。

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn mark_interrupted(_signal: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

/// 为当前 CLI 进程登记 Ctrl-C 取消；调用方须在执行原生任务前登记一次。
#[cfg(unix)]
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

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT};

    #[test]
    fn control_callback_marks_cancellation_only_for_interrupts() {
        INTERRUPTED.store(false, Ordering::Relaxed);
        assert_eq!(unsafe { mark_console_interrupted(CTRL_CLOSE_EVENT) }, 0);
        assert!(!sigint_cancelled());
        assert_eq!(unsafe { mark_console_interrupted(CTRL_C_EVENT) }, 1);
        assert!(sigint_cancelled());
        INTERRUPTED.store(false, Ordering::Relaxed);
        assert_eq!(unsafe { mark_console_interrupted(CTRL_BREAK_EVENT) }, 1);
        assert!(sigint_cancelled());
        INTERRUPTED.store(false, Ordering::Relaxed);
    }
}

#[cfg(windows)]
unsafe extern "system" fn mark_console_interrupted(event: u32) -> windows_sys::core::BOOL {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    if event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT {
        INTERRUPTED.store(true, Ordering::Relaxed);
        1
    } else {
        // Closing a console/logoff cannot be converted into an unbounded graceful
        // shutdown. Keep the OS default termination and Job Object close cleanup.
        0
    }
}

/// Register the Windows Ctrl-C/Break bridge. Failure is returned to the caller;
/// no console trust or process privilege is changed.
#[cfg(windows)]
pub fn install_sigint_cancellation() -> io::Result<()> {
    INTERRUPTED.store(false, Ordering::Relaxed);
    // SAFETY: static system-ABI callback touches only an atomic flag.
    if unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
            Some(mark_console_interrupted),
            1,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
