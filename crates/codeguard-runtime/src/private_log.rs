//! Unix 私有原始证据日志；只在调用者预先创建的受管目录中写入。

use crate::{ProcessOutcome, ProcessSpec, Termination, run_process};
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// 本轮原生执行独占的报告槽位；仅证明指定路径在准备时不存在及读取时为有界普通文件。
pub struct FreshReportSlot {
    directory: File,
    name: CString,
    max_bytes: u64,
}

impl FreshReportSlot {
    /// 读取本轮执行后创建的报告；缺失、链接、特殊文件和过大报告均为错误。
    pub fn read(&self) -> io::Result<Vec<u8>> {
        // SAFETY: directory 是已打开的私有目录，name 为单个有效文件名；拒绝末尾链接。
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat 成功返回由本函数独占的文件描述符。
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file()
            || metadata.nlink() != 1
            || metadata.len() > self.max_bytes
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "原生报告不是有界普通文件",
            ));
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "原生报告超过大小上限",
            ));
        }
        Ok(bytes)
    }
}

/// 在已存在的私有目录准备原生报告输出，旧报告一律拒绝，绝不清理用户文件。
pub fn prepare_fresh_report(
    root: &Path,
    name: &str,
    max_bytes: u64,
) -> io::Result<FreshReportSlot> {
    if max_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "报告大小上限必须为正数",
        ));
    }
    let name = checked_name(name)?;
    let directory = open_root_without_symlinks(root)?;
    verify_private_root(&directory)?;
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: directory 有效；name 为单个文件名；只检查条目是否已存在。
    let found = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            info.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if found == 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "报告目标已存在，不能复用旧证据",
        ));
    }
    let error = io::Error::last_os_error();
    if error.kind() != io::ErrorKind::NotFound {
        return Err(error);
    }
    Ok(FreshReportSlot {
        directory,
        name,
        max_bytes,
    })
}

/// 原生执行后无法签发完成结果的原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessEvidenceFailureKind {
    /// 必要的私有证据未成功持久化。
    LogWrite,
    /// 进程虽退出，但记录完成时已超过请求截止时间。
    DeadlineExceeded,
}

impl ProcessEvidenceFailureKind {
    /// 返回稳定诊断码，供适配器区分证据失败和请求超时。
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::LogWrite => "evidence_write_failed",
            Self::DeadlineExceeded => "request_deadline_exceeded",
        }
    }
}

/// 原生进程与本轮报告联合留证时的失败阶段。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportEvidenceFailureKind {
    /// 报告路径或私有目录在启动前不满足新鲜度要求。
    Preflight,
    /// 原生进程日志未能完整落盘，或记录时已过截止时间。
    ProcessEvidence,
    /// 本次进程后没有可读取的有界普通报告文件。
    ReportRead,
    /// 报告读取完毕时本次请求已过共同截止时间。
    DeadlineExceeded,
}

/// 本轮进程的原始结果及由同一私有运行区读取的报告字节。
#[derive(Debug)]
pub struct ReportedProcessOutcome {
    /// 原生退出状态和输出；仍需适配器按工具契约解释。
    pub outcome: ProcessOutcome,
    /// 本轮新生成的原生报告；仍需适配器验证格式、范围和规则来源。
    pub report: Vec<u8>,
}

/// 报告或留证失败；已有进程结果必须保留供上层诊断。
#[derive(Debug)]
pub struct ReportEvidenceFailure {
    /// 失败阶段。
    pub kind: ReportEvidenceFailureKind,
    /// 原生进程已启动时保留其退出状态与输出。
    pub outcome: Option<ProcessOutcome>,
    /// 底层 I/O 或截止时间错误。
    pub source: io::Error,
}

/// 先拒绝旧报告，再运行原生命令、记录私有日志并读取本轮报告。
/// 本函数只证明执行顺序与本地报告新鲜度；规则、源码范围及报告内容由适配器核验。
pub fn run_process_recorded_with_report(
    spec: &ProcessSpec,
    cancelled: &AtomicBool,
    evidence_root: &Path,
    log_name: &str,
    report_root: &Path,
    report_name: &str,
    max_report_bytes: u64,
) -> Result<ReportedProcessOutcome, ReportEvidenceFailure> {
    let slot =
        prepare_fresh_report(report_root, report_name, max_report_bytes).map_err(|source| {
            ReportEvidenceFailure {
                kind: ReportEvidenceFailureKind::Preflight,
                outcome: None,
                source,
            }
        })?;
    let outcome =
        run_process_recorded(spec, cancelled, evidence_root, log_name).map_err(|failure| {
            ReportEvidenceFailure {
                kind: ReportEvidenceFailureKind::ProcessEvidence,
                outcome: Some(failure.outcome),
                source: failure.source,
            }
        })?;
    let report = match slot.read() {
        Ok(report) => report,
        Err(source) => {
            return Err(ReportEvidenceFailure {
                kind: ReportEvidenceFailureKind::ReportRead,
                outcome: Some(outcome),
                source,
            });
        }
    };
    if Instant::now() >= spec.deadline {
        return Err(ReportEvidenceFailure {
            kind: ReportEvidenceFailureKind::DeadlineExceeded,
            outcome: Some(outcome),
            source: io::Error::new(io::ErrorKind::TimedOut, "报告读取后请求已超时"),
        });
    }
    Ok(ReportedProcessOutcome { outcome, report })
}

/// 原生检查已经结束，但证据写入失败或完成时超时；不得把其中的退出码当作检查通过。
#[derive(Debug)]
pub struct ProcessEvidenceFailure {
    /// 真实执行结果，供诊断使用。
    pub outcome: ProcessOutcome,
    /// 失败类型，供适配器保留真实诊断。
    pub kind: ProcessEvidenceFailureKind,
    /// I/O 失败或超过总截止时间的错误详情。
    pub source: io::Error,
}

/// 执行原生检查并持久化私有原始证据；持久化失败或完成时超时均返回显式错误。
pub fn run_process_recorded(
    spec: &ProcessSpec,
    cancelled: &AtomicBool,
    root: &Path,
    name: &str,
) -> Result<ProcessOutcome, ProcessEvidenceFailure> {
    let outcome = run_process(spec, cancelled);
    if let Err(source) = write_private_log(root, name, &outcome) {
        return Err(ProcessEvidenceFailure {
            outcome,
            kind: ProcessEvidenceFailureKind::LogWrite,
            source,
        });
    }
    if matches!(outcome.termination, Termination::Exited(_)) && Instant::now() >= spec.deadline {
        return Err(ProcessEvidenceFailure {
            outcome,
            kind: ProcessEvidenceFailureKind::DeadlineExceeded,
            source: io::Error::new(io::ErrorKind::TimedOut, "证据落盘后请求已超时"),
        });
    }
    Ok(outcome)
}

/// 将原始结果写入已有的私有目录，目标不能存在且不能经符号链接重定向。
/// `root` 必须由调用者创建并拥有；`name` 只能是一个普通文件名。
pub fn write_private_log(root: &Path, name: &str, outcome: &ProcessOutcome) -> io::Result<PathBuf> {
    let name = checked_name(name)?;
    let root_dir = open_root_without_symlinks(root)?;
    verify_private_root(&root_dir)?;

    let temporary = CString::new(format!(
        ".codeguard-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ))
    .expect("生成的临时文件名无 NUL");
    // SAFETY: root_fd 指向私有目录；O_EXCL/O_NOFOLLOW 确保只创建新普通文件。
    let temporary_fd = unsafe {
        libc::openat(
            root_dir.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if temporary_fd == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat 返回新拥有的文件描述符。
    let mut file = unsafe { File::from_raw_fd(temporary_fd) };
    let written = write_record(&mut file, outcome).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let _ = unlink_temporary(&root_dir, &temporary);
        return Err(error);
    }
    // 同目录 hard link 是原子且不覆盖已有目标；目标即使是 symlink 也报 EEXIST。
    // SAFETY: 两个目录 fd 均指向同一个已打开的私有目录，名称不含路径分隔符。
    let linked = unsafe {
        libc::linkat(
            root_dir.as_raw_fd(),
            temporary.as_ptr(),
            root_dir.as_raw_fd(),
            name.as_ptr(),
            0,
        )
    };
    let link_error = if linked == -1 {
        Some(io::Error::last_os_error())
    } else {
        None
    };
    let unlink_error = unlink_temporary(&root_dir, &temporary).err();
    if let Some(error) = link_error {
        return Err(error);
    }
    if let Some(error) = unlink_error {
        return Err(error);
    }
    root_dir.sync_all()?;
    Ok(root.join(name.to_str().expect("输入文件名为 UTF-8")))
}

fn open_root_without_symlinks(root: &Path) -> io::Result<File> {
    let mut components = root.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "证据目录必须使用绝对路径",
        ));
    }
    // SAFETY: 固定根目录路径有效；返回的 fd 立即交给 File 管理。
    let root_fd = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        )
    };
    if root_fd == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: open 成功时返回独占的文件描述符。
    let mut current = unsafe { File::from_raw_fd(root_fd) };
    for component in components {
        let Component::Normal(segment) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "证据目录路径不可包含父目录或特殊分量",
            ));
        };
        let segment = CString::new(segment.as_bytes()).map_err(invalid_path)?;
        // SAFETY: 当前 fd 是已打开的真实目录；每一级拒绝符号链接。
        let next_fd = unsafe {
            libc::openat(
                current.as_raw_fd(),
                segment.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            )
        };
        if next_fd == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat 成功时返回独占的文件描述符。
        current = unsafe { File::from_raw_fd(next_fd) };
    }
    Ok(current)
}

fn checked_name(name: &str) -> io::Result<CString> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
        || name.contains('/')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "日志名称必须是单个文件名",
        ));
    }
    CString::new(name).map_err(invalid_path)
}

fn invalid_path(_: std::ffi::NulError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "路径包含 NUL")
}

fn verify_private_root(dir: &File) -> io::Result<()> {
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fd 有效，fstat 成功时会完整初始化 stat。
    let result = unsafe { libc::fstat(dir.as_raw_fd(), info.as_mut_ptr()) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: 前面的 fstat 已成功。
    let info = unsafe { info.assume_init() };
    // SAFETY: 读取当前进程的有效用户身份，不操作外部状态。
    let current_uid = unsafe { libc::geteuid() };
    if info.st_uid != current_uid || info.st_mode & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "证据目录必须属于当前用户且不可被其他用户访问",
        ));
    }
    Ok(())
}

fn write_record(file: &mut File, outcome: &ProcessOutcome) -> io::Result<()> {
    file.write_all(b"CGLOG1\0")?;
    let (kind, exit_code) = termination_code(outcome.termination);
    file.write_all(&[kind])?;
    file.write_all(&exit_code.to_le_bytes())?;
    let elapsed = u64::try_from(outcome.elapsed.as_nanos()).unwrap_or(u64::MAX);
    file.write_all(&elapsed.to_le_bytes())?;
    file.write_all(&(outcome.stdout.len() as u64).to_le_bytes())?;
    file.write_all(&(outcome.stderr.len() as u64).to_le_bytes())?;
    file.write_all(&outcome.stdout)?;
    file.write_all(&outcome.stderr)
}

fn termination_code(termination: Termination) -> (u8, i32) {
    match termination {
        Termination::Exited(code) => (0, code),
        Termination::Signaled => (1, 0),
        Termination::InvalidSpec => (2, 0),
        Termination::DeadlineBeforeStart => (3, 0),
        Termination::Cancelled => (4, 0),
        Termination::TimedOut => (5, 0),
        Termination::OutputLimit => (6, 0),
        Termination::SpawnFailure => (7, 0),
        Termination::ReadFailure => (8, 0),
        Termination::WriteFailure => (9, 0),
        Termination::CleanupFailure => (10, 0),
        Termination::UnsupportedPlatform => (11, 0),
    }
}

fn unlink_temporary(dir: &File, name: &CString) -> io::Result<()> {
    // SAFETY: 只删除本次创建的同目录临时名称；清理失败不允许清理其它路径。
    if unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
