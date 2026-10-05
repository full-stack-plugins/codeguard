#![cfg(unix)]

use codeguard_runtime::{
    ProcessEvidenceFailureKind, ProcessOutcome, ProcessSpec, Termination, run_process_recorded,
    write_private_log,
};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir()
            .canonicalize()
            .expect("规范化临时目录")
            .join(format!(
                "codeguard-private-log-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&path).expect("创建测试目录");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("私有目录");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn outcome() -> ProcessOutcome {
    ProcessOutcome {
        termination: Termination::TimedOut,
        spawn_os_error: None,
        stdout: vec![0xff, 0, b'a'],
        stderr: b"native error\n".to_vec(),
        elapsed: Duration::from_millis(25),
    }
}

#[test]
fn writes_an_atomic_private_record_with_raw_bytes() {
    let dir = TestDir::new();
    let path = write_private_log(&dir.0, "run-1.log", &outcome()).expect("持久化证据");
    assert_eq!(path, dir.0.join("run-1.log"));
    let bytes = fs::read(&path).expect("读取证据");
    assert!(bytes.starts_with(b"CGLOG1\0"));
    assert!(bytes.windows(3).any(|window| window == [0xff, 0, b'a']));
    assert!(bytes.ends_with(b"native error\n"));
    assert_eq!(
        fs::metadata(path).expect("证据元数据").permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn refuses_symlink_destination_without_touching_external_file() {
    let dir = TestDir::new();
    let outside = TestDir::new();
    let target = outside.0.join("outside.txt");
    fs::write(&target, b"untouched").expect("外部文件");
    symlink(&target, dir.0.join("run-1.log")).expect("建立符号链接");
    assert!(write_private_log(&dir.0, "run-1.log", &outcome()).is_err());
    assert_eq!(fs::read(target).expect("外部文件仍存在"), b"untouched");
}

#[test]
fn refuses_symlink_in_root_ancestor_without_writing_outside() {
    let dir = TestDir::new();
    let outside = TestDir::new();
    let target = outside.0.join("logs");
    fs::create_dir(&target).expect("外部日志目录");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).expect("私有权限");
    symlink(&outside.0, dir.0.join("redirect")).expect("建立中间层符号链接");
    let redirected = dir.0.join("redirect/logs");
    assert!(write_private_log(&redirected, "run.log", &outcome()).is_err());
    assert!(!target.join("run.log").exists());
}

#[test]
fn native_exit_zero_with_failed_evidence_write_is_not_a_success_result() {
    let dir = TestDir::new();
    let outside = TestDir::new();
    let target = outside.0.join("outside.txt");
    fs::write(&target, b"untouched").expect("外部文件");
    symlink(&target, dir.0.join("run.log")).expect("建立符号链接");
    let spec = ProcessSpec {
        executable: "/bin/echo".into(),
        args: vec!["native-ok".into()],
        cwd: dir.0.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(2),
        output_limit_bytes: 1024,
    };
    let failure = run_process_recorded(&spec, &AtomicBool::new(false), &dir.0, "run.log")
        .expect_err("证据失败不能返回成功");
    assert_eq!(failure.kind, ProcessEvidenceFailureKind::LogWrite);
    assert_eq!(failure.outcome.termination, Termination::Exited(0));
    assert_eq!(failure.outcome.stdout, b"native-ok\n");
    assert_eq!(fs::read(target).expect("外部文件仍存在"), b"untouched");
}

#[test]
fn refuses_symlink_or_public_root_and_existing_destination() {
    let dir = TestDir::new();
    let root_link = dir.0.join("root-link");
    symlink(&dir.0, &root_link).expect("目录符号链接");
    assert!(write_private_log(&root_link, "run.log", &outcome()).is_err());

    fs::write(dir.0.join("run.log"), b"original").expect("已有日志");
    assert!(write_private_log(&dir.0, "run.log", &outcome()).is_err());
    assert_eq!(
        fs::read(dir.0.join("run.log")).expect("已有日志"),
        b"original"
    );

    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).expect("改目录权限");
    assert!(write_private_log(&dir.0, "new.log", &outcome()).is_err());
}

#[test]
fn refuses_path_components_in_log_name() {
    let dir = TestDir::new();
    for name in ["../escape.log", "sub/run.log", ".", "..", ""] {
        assert!(
            write_private_log(&dir.0, name, &outcome()).is_err(),
            "{name:?}"
        );
    }
}

#[test]
fn spawn_failure_retains_os_error_without_fabricating_native_output() {
    let dir = TestDir::new();
    let spec = ProcessSpec {
        executable: dir.0.join("missing-tool"),
        args: Vec::new(),
        cwd: dir.0.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(2),
        output_limit_bytes: 1024,
    };
    let result = run_process_recorded(&spec, &AtomicBool::new(false), &dir.0, "spawn.log")
        .expect("启动失败仍需记录私有证据");
    assert_eq!(result.termination, Termination::SpawnFailure);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    let bytes = fs::read(dir.0.join("spawn.log")).unwrap();
    assert_eq!(bytes[7], 7);
    assert_eq!(
        i32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        libc::ENOENT
    );
}
