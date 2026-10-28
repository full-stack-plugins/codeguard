//! Native Windows-only resource and fail-closed contracts; never simulated on Unix.
#![cfg(windows)]
use codeguard_runtime::{ProcessSpec, Termination, run_process};
use std::{
    collections::BTreeMap,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

fn spec() -> ProcessSpec {
    ProcessSpec {
        executable: std::env::current_exe().unwrap(),
        args: ["--exact", "windows_probe_child", "--nocapture"]
            .map(Into::into)
            .to_vec(),
        cwd: std::env::current_dir().unwrap(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(5),
        output_limit_bytes: 4096,
    }
}

#[test]
fn windows_probe_child() {}

fn handles() -> u32 {
    let mut count = 0;
    assert_ne!(
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) },
        0
    );
    count
}

// Run serially inside this test binary: other tests only count their own handles
// transiently, and the tolerance is bounded rather than proportional to iterations.
#[test]
fn successful_and_failed_launches_release_handles() {
    assert_eq!(
        run_process(&spec(), &AtomicBool::new(false)).termination,
        Termination::Exited(0)
    );
    let before = handles();
    for _ in 0..32 {
        assert_eq!(
            run_process(&spec(), &AtomicBool::new(false)).termination,
            Termination::Exited(0)
        );
        let mut missing = spec();
        missing.executable = missing.cwd.join("codeguard-guaranteed-missing-process.exe");
        assert!(!missing.executable.exists());
        assert_eq!(
            run_process(&missing, &AtomicBool::new(false)).termination,
            Termination::SpawnFailure
        );
    }
    assert!(
        handles() <= before + 4,
        "process/job/thread/pipe handles leaked"
    );
}

#[test]
fn batch_paths_cannot_implicitly_invoke_a_shell() {
    for extension in ["cmd", "CMD", "bat", "BAT"] {
        let mut request = spec();
        request.executable.set_extension(extension);
        assert_eq!(
            run_process(&request, &AtomicBool::new(false)).termination,
            Termination::InvalidSpec
        );
    }
}
