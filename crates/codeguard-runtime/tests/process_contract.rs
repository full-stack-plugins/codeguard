#![cfg(unix)]

use codeguard_runtime::{ProcessSpec, Termination, run_process};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn spec(executable: &str, args: &[&str]) -> ProcessSpec {
    ProcessSpec {
        executable: PathBuf::from(executable),
        args: args.iter().map(OsString::from).collect(),
        cwd: std::env::temp_dir(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(2),
        output_limit_bytes: 1024,
    }
}

#[test]
fn argv_metacharacters_are_literal_and_cannot_spawn_a_shell() {
    let marker =
        std::env::temp_dir().join(format!("codeguard-never-created-{}", std::process::id()));
    assert!(!marker.exists());
    let payload = format!("$(touch {})", marker.display());
    let request = spec("/bin/echo", &[&payload, "a;b", "white space"]);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::Exited(0));
    assert_eq!(
        String::from_utf8_lossy(&actual.stdout),
        format!("{payload} a;b white space\n")
    );
    assert!(!marker.exists());
}

#[test]
fn explicit_stdin_is_supplied_without_inheriting_terminal_input() {
    let mut request = spec("/bin/cat", &[]);
    request.stdin = Some(b"literal input\n".to_vec());
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::Exited(0));
    assert_eq!(actual.stdout, b"literal input\n");

    request.stdin = None;
    let closed = run_process(&request, &AtomicBool::new(false));
    assert_eq!(closed.termination, Termination::Exited(0));
    assert!(closed.stdout.is_empty());
}

#[test]
fn output_flood_is_bounded_and_reported_incomplete() {
    let mut request = spec("/usr/bin/yes", &[]);
    request.output_limit_bytes = 128;
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::OutputLimit);
    assert!(actual.stdout.len() + actual.stderr.len() <= 128);
}

#[test]
fn finite_output_over_limit_cannot_become_success_after_child_exits() {
    let mut request = spec(
        "/bin/sh",
        &["-c", "printf '123456789' ; printf 'abcdefghi' >&2"],
    );
    request.output_limit_bytes = 8;
    let actual = run_process(&request, &AtomicBool::new(false));
    assert!(
        matches!(
            actual.termination,
            Termination::OutputLimit | Termination::CleanupFailure
        ),
        "有限超限输出不可被解释为成功：{:?}",
        actual.termination
    );
    assert!(actual.stdout.len() + actual.stderr.len() <= 8);
}

#[test]
fn simultaneous_stdout_and_stderr_flood_share_one_budget() {
    let mut request = spec("/bin/sh", &["-c", "yes stdout & yes stderr >&2 & wait"]);
    request.output_limit_bytes = 256;
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::OutputLimit);
    assert!(actual.stdout.len() + actual.stderr.len() <= 256);
    assert!(actual.elapsed < Duration::from_secs(2));
}

#[test]
fn expired_deadline_prevents_spawn_and_live_deadline_kills_group() {
    let mut request = spec("/bin/sleep", &["5"]);
    request.deadline = Instant::now() - Duration::from_millis(1);
    assert_eq!(
        run_process(&request, &AtomicBool::new(false)).termination,
        Termination::DeadlineBeforeStart
    );
    request.deadline = Instant::now() + Duration::from_millis(80);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::TimedOut);
    assert!(actual.elapsed < Duration::from_secs(2));
}

#[test]
fn cancellation_is_distinct_from_timeout() {
    let request = spec("/bin/sleep", &["5"]);
    let cancelled = AtomicBool::new(true);
    let actual = run_process(&request, &cancelled);
    assert_eq!(actual.termination, Termination::Cancelled);
    cancelled.store(false, Ordering::Relaxed);
}

#[test]
fn cancellation_during_execution_stops_the_process_group() {
    let request = spec("/bin/sleep", &["5"]);
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        signal.store(true, Ordering::Relaxed);
    });
    let actual = run_process(&request, &cancelled);
    trigger.join().expect("取消信号线程");
    assert_eq!(actual.termination, Termination::Cancelled);
    assert!(actual.elapsed < Duration::from_secs(2));
}

#[test]
fn relative_executable_is_rejected_before_spawn() {
    let request = spec("echo", &["hello"]);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::InvalidSpec);
}

#[test]
fn cwd_and_environment_are_explicit_and_do_not_inherit_parent_values() {
    let mut request = spec("/usr/bin/env", &[]);
    request
        .env
        .insert("CODEGUARD_TEST_ONLY".into(), "literal=value".into());
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::Exited(0));
    assert_eq!(actual.stdout, b"CODEGUARD_TEST_ONLY=literal=value\n");

    let mut cwd_request = spec("/bin/pwd", &[]);
    cwd_request.cwd = std::env::temp_dir().canonicalize().expect("临时目录");
    let actual = run_process(&cwd_request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::Exited(0));
    assert_eq!(
        String::from_utf8_lossy(&actual.stdout).trim(),
        cwd_request.cwd.display().to_string()
    );
}

#[test]
fn spawned_child_in_the_same_process_group_is_reaped_on_timeout() {
    let mut request = spec("/bin/sh", &["-c", "sleep 5 & wait"]);
    request.deadline = Instant::now() + Duration::from_millis(80);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::TimedOut);
    assert!(actual.elapsed < Duration::from_secs(2));
}

#[test]
fn timed_out_process_tree_cannot_write_after_the_request_returns() {
    let marker = std::env::temp_dir().join(format!(
        "codeguard-timeout-grandchild-{}-{:?}",
        std::process::id(),
        thread::current().id()
    ));
    let _ = fs::remove_file(&marker);
    let mut request = spec(
        "/bin/sh",
        &[
            "-c",
            "(printf started; sleep 0.4; touch \"$CODEGUARD_MARKER\") & wait",
        ],
    );
    request
        .env
        .insert("CODEGUARD_MARKER".into(), marker.as_os_str().to_os_string());
    request.deadline = Instant::now() + Duration::from_millis(200);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::TimedOut);
    assert_eq!(actual.stdout, b"started");
    thread::sleep(Duration::from_millis(350));
    assert!(!marker.exists(), "超时返回后子进程仍产生副作用");
}

#[test]
fn cancelled_process_tree_cannot_write_after_the_request_returns() {
    let marker = std::env::temp_dir().join(format!(
        "codeguard-cancel-grandchild-{}-{:?}",
        std::process::id(),
        thread::current().id()
    ));
    let _ = fs::remove_file(&marker);
    let mut request = spec(
        "/bin/sh",
        &[
            "-c",
            "(printf started; sleep 0.4; touch \"$CODEGUARD_MARKER\") & wait",
        ],
    );
    request
        .env
        .insert("CODEGUARD_MARKER".into(), marker.as_os_str().to_os_string());
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        signal.store(true, Ordering::Relaxed);
    });
    let actual = run_process(&request, &cancelled);
    trigger.join().unwrap();
    assert_eq!(actual.termination, Termination::Cancelled);
    assert_eq!(actual.stdout, b"started");
    thread::sleep(Duration::from_millis(350));
    assert!(!marker.exists(), "取消返回后子进程仍产生副作用");
}

#[test]
fn timeout_retains_partial_output_as_incomplete_evidence() {
    let mut request = spec("/bin/sh", &["-c", "printf 'before-timeout'; sleep 5"]);
    request.deadline = Instant::now() + Duration::from_millis(80);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::TimedOut);
    assert_eq!(actual.stdout, b"before-timeout");
    assert!(actual.elapsed < Duration::from_secs(2));
}

#[test]
fn blocked_stdin_writer_does_not_outlive_the_request_deadline() {
    let mut request = spec("/bin/sleep", &["5"]);
    request.stdin = Some(vec![b'x'; 8 * 1024 * 1024]);
    request.deadline = Instant::now() + Duration::from_millis(80);
    let actual = run_process(&request, &AtomicBool::new(false));
    assert_eq!(actual.termination, Termination::TimedOut);
    assert!(actual.elapsed < Duration::from_secs(2));
}
