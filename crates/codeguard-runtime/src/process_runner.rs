//! Rust 原生进程执行内核；不解释检查规则，也不调用 Shell。

use crate::{ProcessSpec, Termination};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 进程原始结果，公开报告前必须另行脱敏和持久化。
#[derive(Clone, Debug)]
pub struct ProcessOutcome {
    /// 真实终止原因。
    pub termination: Termination,
    /// 截断到共同预算的原始 stdout。
    pub stdout: Vec<u8>,
    /// 截断到共同预算的原始 stderr。
    pub stderr: Vec<u8>,
    /// 本次执行实际耗时。
    pub elapsed: Duration,
}

impl ProcessOutcome {
    fn empty(termination: Termination, started: Instant) -> Self {
        Self {
            termination,
            stdout: Vec::new(),
            stderr: Vec::new(),
            elapsed: started.elapsed(),
        }
    }
}

/// 运行一条受控原生命令；取消、超时和超限时停止 Unix 进程组。
#[must_use]
pub fn run_process(spec: &ProcessSpec, cancelled: &AtomicBool) -> ProcessOutcome {
    let started = Instant::now();
    if cancellation_requested(cancelled) {
        return ProcessOutcome::empty(Termination::Cancelled, started);
    }
    if spec.deadline <= started {
        return ProcessOutcome::empty(Termination::DeadlineBeforeStart, started);
    }
    if !spec.executable.is_absolute()
        || !spec.cwd.is_absolute()
        || spec.output_limit_bytes == 0
        || spec
            .stdin
            .as_ref()
            .is_some_and(|bytes| bytes.len() > 8 * 1024 * 1024)
    {
        return ProcessOutcome::empty(Termination::InvalidSpec, started);
    }
    #[cfg(not(unix))]
    {
        let _ = spec;
        return ProcessOutcome::empty(Termination::UnsupportedPlatform, started);
    }
    #[cfg(unix)]
    run_unix(spec, cancelled, started)
}

#[cfg(unix)]
fn run_unix(spec: &ProcessSpec, cancelled: &AtomicBool, started: Instant) -> ProcessOutcome {
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .envs(&spec.env)
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return ProcessOutcome::empty(Termination::SpawnFailure, started),
    };
    let stdout = child.stdout.take().expect("stdout 已配置 pipe");
    let stderr = child.stderr.take().expect("stderr 已配置 pipe");
    if set_nonblocking(&stdout).is_err() || set_nonblocking(&stderr).is_err() {
        let _ = kill_group(child.id());
        let _ = child.kill();
        let _ = child.wait();
        return ProcessOutcome::empty(Termination::ReadFailure, started);
    }
    let remaining = Arc::new(Mutex::new(spec.output_limit_bytes));
    let over_limit = Arc::new(AtomicBool::new(false));
    let drain_deadline = Arc::new(Mutex::new(None));
    let stdout_reader = spawn_reader(
        stdout,
        Arc::clone(&remaining),
        Arc::clone(&over_limit),
        Arc::clone(&drain_deadline),
    );
    let stderr_reader = spawn_reader(
        stderr,
        Arc::clone(&remaining),
        Arc::clone(&over_limit),
        Arc::clone(&drain_deadline),
    );
    let stdin_writer = spec.stdin.as_ref().map(|bytes| {
        let pipe = child.stdin.take().expect("stdin 已配置 pipe");
        let configured = set_nonblocking(&pipe);
        let bytes = bytes.clone();
        let drain_deadline = Arc::clone(&drain_deadline);
        thread::spawn(move || configured.is_ok() && write_stdin(pipe, &bytes, drain_deadline))
    });

    let termination = loop {
        if cancellation_requested(cancelled) {
            break Termination::Cancelled;
        }
        if over_limit.load(Ordering::Relaxed) {
            break Termination::OutputLimit;
        }
        if Instant::now() >= spec.deadline {
            break Termination::TimedOut;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break status
                    .code()
                    .map_or(Termination::Signaled, Termination::Exited);
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(_) => break Termination::CleanupFailure,
        }
    };
    // 进程组仍可能持有管道，即使直接子进程已经退出；统一停止后再回收。
    let cleanup_ok = kill_group(child.id());
    if let Ok(mut deadline) = drain_deadline.lock() {
        // 读线程与子进程退出之间可能有调度延迟；排空仍受请求的绝对期限约束。
        // 固定 150ms 会在并行检查负载下把已退出的短命令误判为读取失败。
        *deadline = Some(spec.deadline);
    }
    if !cleanup_ok {
        let _ = child.kill();
    }
    let wait_ok = child.wait().is_ok();
    let stdout_result = stdout_reader.join().ok();
    let stderr_result = stderr_reader.join().ok();
    let write_ok = stdin_writer
        .map(|writer| writer.join().unwrap_or(false))
        .unwrap_or(true);
    let termination = finalize_termination(
        termination,
        cleanup_ok,
        wait_ok,
        over_limit.load(Ordering::Relaxed),
        stdout_result.as_ref().is_some_and(|result| result.complete),
        stderr_result.as_ref().is_some_and(|result| result.complete),
        write_ok,
        Instant::now() >= spec.deadline,
    );
    ProcessOutcome {
        termination,
        stdout: stdout_result.map_or_else(Vec::new, |result| result.bytes),
        stderr: stderr_result.map_or_else(Vec::new, |result| result.bytes),
        elapsed: started.elapsed(),
    }
}

fn cancellation_requested(cancelled: &AtomicBool) -> bool {
    if cancelled.load(Ordering::Relaxed) {
        return true;
    }
    #[cfg(unix)]
    {
        crate::interrupt::sigint_cancelled()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn finalize_termination(
    initial: Termination,
    cleanup_ok: bool,
    wait_ok: bool,
    over_limit: bool,
    stdout_complete: bool,
    stderr_complete: bool,
    write_ok: bool,
    deadline_expired: bool,
) -> Termination {
    if !cleanup_ok || !wait_ok {
        Termination::CleanupFailure
    } else if !matches!(initial, Termination::Exited(_)) {
        initial
    } else if over_limit {
        // 子进程可能先退出，读线程随后才发现有限输出超过共同预算。
        Termination::OutputLimit
    } else if deadline_expired {
        // 直接子进程退出不代表读取、清理已在总截止时间内完成。
        Termination::TimedOut
    } else if !stdout_complete || !stderr_complete {
        Termination::ReadFailure
    } else if !write_ok {
        Termination::WriteFailure
    } else {
        initial
    }
}

struct ReaderOutput {
    bytes: Vec<u8>,
    complete: bool,
}

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    remaining: Arc<Mutex<usize>>,
    over_limit: Arc<AtomicBool>,
    drain_deadline: Arc<Mutex<Option<Instant>>>,
) -> thread::JoinHandle<ReaderOutput> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    let expired = drain_deadline
                        .lock()
                        .map(|deadline| deadline.is_some_and(|time| Instant::now() >= time))
                        .unwrap_or(true);
                    if expired {
                        return ReaderOutput {
                            bytes: output,
                            complete: false,
                        };
                    }
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(_) => {
                    return ReaderOutput {
                        bytes: output,
                        complete: false,
                    };
                }
            };
            if count == 0 {
                break;
            }
            let Ok(mut budget) = remaining.lock() else {
                return ReaderOutput {
                    bytes: output,
                    complete: false,
                };
            };
            let retained = count.min(*budget);
            output.extend_from_slice(&buffer[..retained]);
            *budget -= retained;
            if retained < count {
                over_limit.store(true, Ordering::Relaxed);
            }
        }
        ReaderOutput {
            bytes: output,
            complete: true,
        }
    })
}

#[cfg(unix)]
fn write_stdin<W: Write>(
    mut writer: W,
    bytes: &[u8],
    drain_deadline: Arc<Mutex<Option<Instant>>>,
) -> bool {
    let mut offset = 0;
    while offset < bytes.len() {
        match writer.write(&bytes[offset..]) {
            Ok(0) => return false,
            Ok(count) => offset += count,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let expired = drain_deadline
                    .lock()
                    .map(|deadline| deadline.is_some_and(|time| Instant::now() >= time))
                    .unwrap_or(true);
                if expired {
                    return false;
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => return false,
        }
    }
    true
}

#[cfg(unix)]
fn set_nonblocking<T: std::os::fd::AsRawFd>(pipe: &T) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    // SAFETY: fd 属于此进程持有的 ChildStdout/ChildStderr，fcntl 只调整该 pipe 的状态标志。
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd 仍由 pipe 持有，F_SETFL 仅加入 O_NONBLOCK。
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn kill_group(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: 该 pid 来自我们刚创建、已设为独立进程组的子进程；负值只指向此组。
    let result = unsafe { libc::kill(-pid, libc::SIGKILL) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

#[cfg(all(test, unix))]
mod tests {
    use super::finalize_termination;
    use crate::Termination;

    #[test]
    fn exited_child_cannot_report_success_when_drain_crosses_deadline() {
        assert_eq!(
            finalize_termination(
                Termination::Exited(0),
                true,
                true,
                false,
                true,
                true,
                true,
                true
            ),
            Termination::TimedOut
        );
    }

    #[test]
    fn late_output_limit_is_preserved_even_when_deadline_also_expires() {
        assert_eq!(
            finalize_termination(
                Termination::Exited(0),
                true,
                true,
                true,
                true,
                true,
                true,
                true
            ),
            Termination::OutputLimit
        );
    }
}
