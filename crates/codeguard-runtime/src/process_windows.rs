//! Windows process containment. No child code runs before Job Object assignment.
//! Writes use PIPE_NOWAIT; read handles are probed with PeekNamedPipe before reading.
//! All I/O is polled on the controller thread; there are no blocking drain threads.
use super::*;
use std::mem::{size_of, zeroed};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_NO_MORE_FILES, GetLastError, HANDLE,
        INVALID_HANDLE_VALUE,
    },
    System::{
        Diagnostics::ToolHelp::*,
        JobObjects::*,
        Pipes::{PIPE_NOWAIT, PeekNamedPipe, SetNamedPipeHandleState},
        Threading::{
            CREATE_SUSPENDED, GetProcessIdOfThread, OpenThread, ResumeThread,
            THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
        },
    },
};

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: this wrapper exclusively owns a valid Win32 handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Handle {
    fn new(raw: HANDLE) -> io::Result<Self> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}

fn job() -> io::Result<Handle> {
    // SAFETY: unnamed, non-inheritable job; initialized structure has exact API size.
    unsafe {
        let job = Handle::new(CreateJobObjectW(std::ptr::null(), std::ptr::null()))?;
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }
}

fn resume_initial_thread(pid: u32) -> io::Result<()> {
    // The process is suspended and its live Child handle prevents PID reuse. It
    // has never executed user code. Require exactly one thread; ambiguity fails
    // closed. Snapshot and opened thread handles are closed on every path.
    unsafe {
        let snapshot = Handle::new(CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0))?;
        let mut entry: THREADENTRY32 = zeroed();
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot.0, &mut entry) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut found = None;
        loop {
            if entry.th32OwnerProcessID == pid {
                if found.is_some() {
                    return Err(io::Error::other("ambiguous suspended child threads"));
                }
                found = Some(entry.th32ThreadID);
            }
            if Thread32Next(snapshot.0, &mut entry) == 0 {
                if GetLastError() != ERROR_NO_MORE_FILES {
                    return Err(io::Error::last_os_error());
                }
                break;
            }
        }
        let id = found.ok_or_else(|| io::Error::other("suspended child thread missing"))?;
        let thread = Handle::new(OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
            0,
            id,
        ))?;
        if GetProcessIdOfThread(thread.0) != pid {
            return Err(io::Error::other("thread ownership changed"));
        }
        if ResumeThread(thread.0) != 1 {
            return Err(io::Error::other("unexpected child suspend count"));
        }
    }
    Ok(())
}

fn nonblocking(pipe: &impl AsRawHandle) -> io::Result<()> {
    let mode = PIPE_NOWAIT;
    // SAFETY: borrowed pipe remains owned by Child*; only its server wait mode changes.
    if unsafe {
        SetNamedPipeHandleState(
            pipe.as_raw_handle(),
            &mode,
            std::ptr::null(),
            std::ptr::null(),
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn empty_job(job: &Handle) -> bool {
    unsafe {
        let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = zeroed();
        QueryInformationJobObject(
            job.0,
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            std::ptr::null_mut(),
        ) != 0
            && accounting.ActiveProcesses == 0
    }
}

struct Drain {
    bytes: Vec<u8>,
    eof: bool,
    failed: bool,
}
impl Drain {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            eof: false,
            failed: false,
        }
    }
    // One bounded read per iteration, so a flooding child cannot starve deadline checks.
    fn poll(&mut self, pipe: &mut (impl Read + AsRawHandle), budget: &mut usize, over: &mut bool) {
        if self.eof || self.failed {
            return;
        }
        // Rust's ChildStdout/ChildStderr handles have GENERIC_READ but not
        // FILE_WRITE_ATTRIBUTES, so changing their wait mode is access denied.
        // PeekNamedPipe is non-consuming. This controller is their sole reader;
        // bytes observed here cannot be consumed by a concurrent reader before
        // our read. Limit the read to that count, never wait for future output.
        let mut available = 0u32;
        if unsafe {
            PeekNamedPipe(
                pipe.as_raw_handle(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } == 0
        {
            if io::Error::last_os_error().raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                self.eof = true;
            } else {
                self.failed = true;
            }
            return;
        }
        if available == 0 {
            return;
        }
        let mut bytes = [0; 4096];
        let requested = (available as usize).min(bytes.len());
        match pipe.read(&mut bytes[..requested]) {
            Ok(0) => self.eof = true,
            Ok(count) => {
                let retained = count.min(*budget);
                self.bytes.extend_from_slice(&bytes[..retained]);
                *budget -= retained;
                *over |= retained < count;
            }
            Err(e) if e.raw_os_error() == Some(ERROR_NO_DATA as i32) => (),
            Err(e) if e.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => self.eof = true,
            Err(_) => self.failed = true,
        }
    }
}

pub(super) fn run(spec: &ProcessSpec, cancelled: &AtomicBool, started: Instant) -> ProcessOutcome {
    run_prepared(spec, cancelled, started, || true)
}

// Private injection point exercises cleanup after suspended creation and job
// assignment. Production callers cannot select a fault through env or arguments.
fn run_prepared(
    spec: &ProcessSpec,
    cancelled: &AtomicBool,
    started: Instant,
    before_resume: impl FnOnce() -> bool,
) -> ProcessOutcome {
    let failure = |error: io::Error| {
        let mut result = ProcessOutcome::empty(Termination::SpawnFailure, started);
        result.spawn_os_error = error.raw_os_error();
        result
    };
    let job = match job() {
        Ok(job) => job,
        Err(e) => return failure(e),
    };
    // .bat/.cmd invoke cmd.exe implicitly in std; this backend promises literal
    // argv and never a shell. These must be invoked via an explicit interpreter.
    if spec
        .executable
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
    {
        return ProcessOutcome::empty(Termination::InvalidSpec, started);
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .envs(&spec.env)
        .creation_flags(CREATE_SUSPENDED)
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => return failure(e),
    };
    let mut stdout = child.stdout.take().expect("configured stdout");
    let mut stderr = child.stderr.take().expect("configured stderr");
    let mut stdin = child.stdin.take();
    // Only the writable parent stdin handle has the rights needed to set
    // PIPE_NOWAIT. Read handles use non-consuming availability probes instead.
    let configured = stdin.as_ref().map_or(Ok(()), nonblocking);
    let assigned = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle()) } != 0;
    let assignment_error = (!assigned).then(io::Error::last_os_error);
    #[cfg(test)]
    eprintln!(
        "suspended process setup: pipe_configuration={configured:?}; job_assignment={assignment_error:?}"
    );
    let setup = configured.and_then(|_| match assignment_error {
        Some(error) => Err(error),
        None if !before_resume() => Err(io::Error::other("injected pre-resume setup failure")),
        None => resume_initial_thread(child.id()),
    });
    if let Err(setup_error) = setup {
        // Preserve the native failure before cleanup changes GetLastError.
        let setup_os_error = setup_error.raw_os_error();
        #[cfg(test)]
        eprintln!("suspended process setup failed: {setup_error:?}; job_assigned={assigned}");
        // Child is still suspended unless ResumeThread returned an unexpected
        // state. Kill explicitly even if assignment failed; never let it run.
        let killed = child.kill().is_ok();
        if assigned {
            unsafe {
                TerminateJobObject(job.0, 1);
            }
        }
        let cleanup_deadline = Instant::now() + Duration::from_secs(1);
        let waited = loop {
            match child.try_wait() {
                Ok(Some(_)) => break true,
                Err(_) => break false,
                Ok(None) if Instant::now() >= cleanup_deadline => break false,
                Ok(None) => thread::sleep(Duration::from_millis(2)),
            }
        };
        let mut result = ProcessOutcome::empty(
            if killed && waited && (!assigned || empty_job(&job)) {
                Termination::SpawnFailure
            } else {
                Termination::CleanupFailure
            },
            started,
        );
        result.spawn_os_error = setup_os_error;
        return result;
    }
    let mut out = Drain::new();
    let mut err = Drain::new();
    let mut budget = spec.output_limit_bytes;
    let mut over = false;
    let mut written = 0;
    let mut write_ok = true;
    let initial = loop {
        if cancellation_requested(cancelled) {
            break Termination::Cancelled;
        }
        if Instant::now() >= spec.deadline {
            break Termination::TimedOut;
        }
        out.poll(&mut stdout, &mut budget, &mut over);
        err.poll(&mut stderr, &mut budget, &mut over);
        if over {
            break Termination::OutputLimit;
        }
        if out.failed || err.failed {
            break Termination::ReadFailure;
        }
        if let (Some(pipe), Some(bytes)) = (stdin.as_mut(), spec.stdin.as_ref()) {
            match pipe.write(&bytes[written..bytes.len().min(written + 4096)]) {
                Ok(count) => written += count,
                Err(e) if e.raw_os_error() == Some(ERROR_NO_DATA as i32) => (),
                Err(_) => {
                    write_ok = false;
                    stdin.take();
                }
            }
            if written == bytes.len() {
                stdin.take();
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break status
                    .code()
                    .map_or(Termination::Signaled, Termination::Exited);
            }
            Err(_) => break Termination::CleanupFailure,
            Ok(None) => thread::sleep(Duration::from_millis(2)),
        }
    };
    // Always terminate the job, including when its original process exited.
    // Breakaway is not enabled; descendant handles cannot extend this lifetime.
    let terminated = unsafe { TerminateJobObject(job.0, 1) } != 0;
    stdin.take();
    let cleanup_deadline = Instant::now() + Duration::from_secs(1);
    let mut waited = false;
    loop {
        out.poll(&mut stdout, &mut budget, &mut over);
        err.poll(&mut stderr, &mut budget, &mut over);
        waited |= matches!(child.try_wait(), Ok(Some(_)));
        if waited && empty_job(&job) && (out.eof || out.failed) && (err.eof || err.failed) {
            break;
        }
        if Instant::now() >= cleanup_deadline {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    write_ok &= spec
        .stdin
        .as_ref()
        .is_none_or(|bytes| written == bytes.len());
    let termination = finalize_termination(
        initial,
        terminated && empty_job(&job),
        waited,
        over,
        out.eof && !out.failed,
        err.eof && !err.failed,
        write_ok,
        Instant::now() >= spec.deadline,
    );
    ProcessOutcome {
        termination,
        spawn_os_error: None,
        stdout: out.bytes,
        stderr: err.bytes,
        elapsed: started.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn suspended_probe_child() {
        if let Some(path) = std::env::var_os("CODEGUARD_SUSPENDED_MARKER") {
            std::fs::write(path, b"child code executed").unwrap();
        }
    }

    #[test]
    fn setup_failure_after_job_assignment_never_runs_child_and_releases_handles() {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
        fn handles() -> u32 {
            let mut n = 0;
            assert_ne!(
                unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut n) },
                0
            );
            n
        }
        let marker = std::env::temp_dir().join(format!(
            "codeguard-suspended-{}-{}.marker",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!marker.exists());
        let before = handles();
        for _ in 0..16 {
            let started = Instant::now();
            let request = ProcessSpec {
                executable: std::env::current_exe().unwrap(),
                args: [
                    "--exact",
                    "process_runner::windows::tests::suspended_probe_child",
                    "--nocapture",
                ]
                .map(Into::into)
                .to_vec(),
                cwd: std::env::current_dir().unwrap(),
                env: BTreeMap::from([(
                    "CODEGUARD_SUSPENDED_MARKER".into(),
                    marker.as_os_str().to_owned(),
                )]),
                stdin: None,
                deadline: started + Duration::from_secs(5),
                output_limit_bytes: 1024,
            };
            let injected = std::cell::Cell::new(false);
            let result = run_prepared(&request, &AtomicBool::new(false), started, || {
                injected.set(true);
                false
            });
            assert!(
                injected.get(),
                "setup failed before post-assignment fault injection: {result:?}"
            );
            assert_eq!(result.termination, Termination::SpawnFailure, "{result:?}");
            assert!(result.elapsed < Duration::from_secs(3));
            assert!(
                !marker.exists(),
                "suspended child executed before containment setup completed"
            );
        }
        assert!(
            handles() <= before + 4,
            "failed setup leaked process/job/thread/pipe handles"
        );
    }
}
