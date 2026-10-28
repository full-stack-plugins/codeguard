//! Native process contracts using this test executable, with no shell fixtures.
//! Windows must execute these tests on Windows; compiling them is not acceptance.

use codeguard_runtime::{ProcessSpec, Termination, run_process};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "codeguard portable space {} {nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }

    fn request(&self, mode: &str) -> ProcessSpec {
        let mut env = BTreeMap::new();
        env.insert("CODEGUARD_PORTABLE_PROBE".into(), mode.into());
        // Windows programs may require the system directory; it is explicit,
        // never permission to inherit the rest of the caller's environment.
        if let Some(root) = std::env::var_os("SystemRoot") {
            env.insert("SystemRoot".into(), root);
        }
        ProcessSpec {
            executable: std::env::current_exe().unwrap(),
            args: ["--exact", "portable_probe_child", "--nocapture", "--quiet"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            cwd: self.0.clone(),
            env,
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(5),
            output_limit_bytes: 4096,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// This is a real native child process, not a mocked ProcessOutcome. Its early
// return in the ordinary test run is intentional; parent tests select a mode.
#[test]
fn portable_probe_child() {
    let Ok(mode) = std::env::var("CODEGUARD_PORTABLE_PROBE") else {
        return;
    };
    match mode.as_str() {
        "literal" => {
            assert_eq!(
                std::env::args_os().last().unwrap(),
                std::env::var_os("EXPECTED_ARG").unwrap()
            );
            assert_eq!(
                fs::canonicalize(std::env::current_dir().unwrap()).unwrap(),
                fs::canonicalize(PathBuf::from(std::env::var_os("EXPECTED_CWD").unwrap())).unwrap()
            );
            assert!(std::env::var_os("PATH").is_none(), "caller PATH leaked");
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).unwrap();
            assert_eq!(input, b"literal stdin\n");
            print!("PORTABLE_LITERAL_OK");
        }
        "empty-stdin" => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            assert!(bytes.is_empty());
            print!("PORTABLE_EMPTY_STDIN_OK");
        }
        "flood" => {
            let writer = thread::spawn(|| {
                let _ = std::io::stderr().write_all(&[b'e'; 128 * 1024]);
            });
            let _ = std::io::stdout().write_all(&[b'o'; 128 * 1024]);
            let _ = writer.join();
        }
        "sleep" => thread::sleep(Duration::from_secs(10)),
        "descendant" | "parent-exits" => {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "portable_probe_child", "--nocapture", "--quiet"])
                .env("CODEGUARD_PORTABLE_PROBE", "delayed-marker")
                .spawn()
                .unwrap();
            fs::write("descendant-started", b"spawn succeeded").unwrap();
            if mode == "descendant" {
                thread::sleep(Duration::from_secs(10));
            }
        }
        "delayed-marker" => {
            thread::sleep(Duration::from_millis(700));
            fs::write("descendant-marker", b"escaped cleanup").unwrap();
        }
        _ => panic!("unknown test probe mode"),
    }
}

#[test]
fn literal_arguments_stdin_cwd_and_explicit_environment_reach_native_child() {
    let fixture = Fixture::new();
    let mut request = fixture.request("literal");
    let value = "spaces ; $(not-a-shell) & | \"quoted\" trailing\\";
    request
        .args
        .extend([OsString::from("--skip"), OsString::from(value)]);
    request.env.insert("EXPECTED_ARG".into(), value.into());
    request
        .env
        .insert("EXPECTED_CWD".into(), fixture.0.as_os_str().to_owned());
    request.stdin = Some(b"literal stdin\n".to_vec());
    let result = run_process(&request, &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::Exited(0), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("PORTABLE_LITERAL_OK"));
}

#[test]
fn absent_stdin_is_closed_instead_of_inherited() {
    let fixture = Fixture::new();
    let result = run_process(&fixture.request("empty-stdin"), &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::Exited(0), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("PORTABLE_EMPTY_STDIN_OK"));
}

#[test]
fn stdout_and_stderr_use_one_retained_output_budget() {
    let fixture = Fixture::new();
    let mut request = fixture.request("flood");
    request.output_limit_bytes = 1024;
    let result = run_process(&request, &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::OutputLimit, "{result:?}");
    assert!(result.stdout.len() + result.stderr.len() <= 1024);
}

#[test]
fn unconsumed_stdin_cannot_keep_the_request_alive_after_its_deadline() {
    let fixture = Fixture::new();
    let mut request = fixture.request("sleep");
    request.stdin = Some(vec![b'x'; 1024 * 1024]);
    request.deadline = Instant::now() + Duration::from_millis(200);
    let result = run_process(&request, &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::TimedOut, "{result:?}");
    assert!(result.elapsed < Duration::from_secs(3));
}

#[test]
fn expired_and_cancelled_requests_do_not_start_a_child() {
    let fixture = Fixture::new();
    let mut request = fixture.request("delayed-marker");
    request.deadline = Instant::now() - Duration::from_millis(1);
    assert_eq!(
        run_process(&request, &AtomicBool::new(false)).termination,
        Termination::DeadlineBeforeStart
    );
    request.deadline = Instant::now() + Duration::from_secs(5);
    assert_eq!(
        run_process(&request, &AtomicBool::new(true)).termination,
        Termination::Cancelled
    );
    thread::sleep(Duration::from_millis(900));
    assert!(!fixture.0.join("descendant-marker").exists());
}

#[test]
fn live_cancellation_terminates_the_native_child() {
    let fixture = Fixture::new();
    let request = fixture.request("descendant");
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        flag.store(true, Ordering::Relaxed);
    });
    let result = run_process(&request, &cancelled);
    trigger.join().unwrap();
    assert_eq!(result.termination, Termination::Cancelled, "{result:?}");
    assert!(result.elapsed < Duration::from_secs(3));
    assert!(fixture.0.join("descendant-started").exists());
    thread::sleep(Duration::from_millis(900));
    assert!(!fixture.0.join("descendant-marker").exists());
}

#[test]
fn timeout_reaps_descendants_before_they_can_write() {
    let fixture = Fixture::new();
    let mut request = fixture.request("descendant");
    request.deadline = Instant::now() + Duration::from_millis(200);
    let result = run_process(&request, &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::TimedOut, "{result:?}");
    assert!(fixture.0.join("descendant-started").exists());
    thread::sleep(Duration::from_millis(900));
    assert!(!fixture.0.join("descendant-marker").exists());
}

#[test]
fn normally_exited_parent_does_not_leave_a_live_descendant() {
    let fixture = Fixture::new();
    let result = run_process(&fixture.request("parent-exits"), &AtomicBool::new(false));
    assert_eq!(result.termination, Termination::Exited(0), "{result:?}");
    assert!(fixture.0.join("descendant-started").exists());
    thread::sleep(Duration::from_millis(900));
    assert!(!fixture.0.join("descendant-marker").exists());
}
