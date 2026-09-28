#![cfg(unix)]
use codeguard_runtime::read_bounded_regular_file;
use std::ffi::CString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn fifo_input_is_rejected_without_waiting_for_a_writer() {
    // 子进程运行同一用例的读取分支，父进程负责超时及回收，避免失败用例挂住整个测试。
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_BOUNDED_FIFO") {
        assert_eq!(
            read_bounded_regular_file(std::path::Path::new(&path), 256 * 1024)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidData
        );
        return;
    }
    let root = std::env::temp_dir().join(format!("cg-bounded-fifo-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fifo = root.join("manifest.json");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "fifo_input_is_rejected_without_waiting_for_a_writer",
            "--nocapture",
        ])
        .env("CODEGUARD_TEST_BOUNDED_FIFO", &fifo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let end = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= end {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        thread::sleep(Duration::from_millis(10));
    };
    fs::remove_dir_all(root).unwrap();
    assert!(
        status.is_some_and(|status| status.success()),
        "FIFO reading blocked or failed; child was reaped"
    );
}
