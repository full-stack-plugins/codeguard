//! 进程组取消：Unix 进程组 + Windows Job Object 的取消/终止/回收
//!
//! 验收标准：子孙进程、超时、Ctrl-C、排队取消有平台实测

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// 取消原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    /// 超时
    Timeout,
    /// Ctrl-C
    CtrlC,
    /// 排队取消
    Queued,
    /// 手动取消
    Manual,
}

/// 取消状态
#[derive(Debug)]
pub struct CancelState {
    /// 是否已取消
    cancelled: AtomicBool,
    /// 取消原因
    reason: Mutex<Option<CancelReason>>,
    /// 取消时间戳
    timestamp: AtomicU64,
}

impl CancelState {
    /// 创建取消状态
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            reason: Mutex::new(None),
            timestamp: AtomicU64::new(0),
        }
    }
    
    /// 触发取消
    pub fn cancel(&self, reason: CancelReason) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut r) = self.reason.lock() {
            *r = Some(reason);
        }
        self.timestamp.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            Ordering::SeqCst,
        );
    }
    
    /// 是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    
    /// 获取取消原因
    pub fn reason(&self) -> Option<CancelReason> {
        self.reason.lock().ok().and_then(|r| *r)
    }
}

impl Default for CancelState {
    fn default() -> Self {
        Self::new()
    }
}

/// 进程组管理器
pub struct ProcessGroupManager {
    /// 取消状态
    cancel_state: CancelState,
    /// 已注册的进程组 ID
    pids: Mutex<Vec<u32>>,
}

impl ProcessGroupManager {
    /// 创建进程组管理器
    pub fn new() -> Self {
        Self {
            cancel_state: CancelState::new(),
            pids: Mutex::new(Vec::new()),
        }
    }
    
    /// 注册进程组
    pub fn register_pgid(&self, pid: u32) {
        if let Ok(mut pids) = self.pids.lock() {
            pids.push(pid);
        }
    }
    
    /// 取消所有进程组
    pub fn cancel_all(&self, reason: CancelReason) {
        self.cancel_state.cancel(reason);
        
        #[cfg(unix)]
        {
            if let Ok(pids) = self.pids.lock() {
                for &pid in pids.iter() {
                    unsafe {
                        // 发送 SIGTERM 到进程组
                        libc::kill(-(pid as i32), libc::SIGTERM);
                    }
                }
            }
        }
        
        #[cfg(windows)]
        {
            // Windows Job Object 终止
            // 实际实现需要 CreateJobObject/AssignProcessToJobObject/TerminateJobObject
        }
    }
    
    /// 超时取消
    pub fn cancel_on_timeout(&self, timeout_ms: u64) {
        let cancel_state = unsafe { &*(&self.cancel_state as *const CancelState) };
        let pids = unsafe { &*(&self.pids as *const Mutex<Vec<u32>>) };
        
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            if !cancel_state.is_cancelled() {
                cancel_state.cancel(CancelReason::Timeout);
                
                #[cfg(unix)]
                {
                    if let Ok(pids) = pids.lock() {
                        for &pid in pids.iter() {
                            unsafe {
                                libc::kill(-(pid as i32), libc::SIGTERM);
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// 获取取消状态
    pub fn cancel_state(&self) -> &CancelState {
        &self.cancel_state
    }
    
    /// 回收进程组
    pub fn reap(&self) {
        #[cfg(unix)]
        {
            if let Ok(pids) = self.pids.lock() {
                for &pid in pids.iter() {
                    unsafe {
                        // 等待子进程
                        libc::waitpid(pid as i32, std::ptr::null_mut(), libc::WNOHANG);
                    }
                }
            }
        }
    }
}

impl Default for ProcessGroupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 排队取消队列
pub struct CancellationQueue {
    /// 待取消的进程组 ID
    queue: Mutex<Vec<u32>>,
}

impl CancellationQueue {
    /// 创建取消队列
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
        }
    }
    
    /// 入队取消
    pub fn enqueue(&self, pid: u32) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(pid);
        }
    }
    
    /// 处理队列
    pub fn process(&self) -> Vec<u32> {
        let mut cancelled = Vec::new();
        if let Ok(mut q) = self.queue.lock() {
            for &pid in q.iter() {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                }
                cancelled.push(pid);
            }
            q.clear();
        }
        cancelled
    }
}

impl Default for CancellationQueue {
    fn default() -> Self {
        Self::new()
    }
}
