#![cfg(unix)]

use codeguard_runtime::{
    ProcessSpec, ReportEvidenceFailureKind, Termination, prepare_fresh_report,
    run_process_recorded_with_report,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
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
                "codeguard-fresh-report-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&path).expect("创建私有报告目录");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("私有目录权限");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn preexisting_report_cannot_be_reused_as_this_runs_evidence() {
    let dir = TestDir::new();
    fs::write(dir.0.join("pmd.xml"), b"<pmd/>").expect("预存报告");
    let error = prepare_fresh_report(&dir.0, "pmd.xml", 1024)
        .err()
        .expect("陈旧报告必须拒绝");
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn fresh_report_can_be_read_only_after_it_is_created() {
    let dir = TestDir::new();
    let slot = prepare_fresh_report(&dir.0, "pmd.xml", 1024).expect("准备新鲜报告槽");
    assert_eq!(
        slot.read().expect_err("未生成报告").kind(),
        ErrorKind::NotFound
    );
    fs::write(dir.0.join("pmd.xml"), b"<pmd version=\"6.15.0\"/>").expect("模拟原生报告输出");
    assert_eq!(
        slot.read().expect("读取本轮报告"),
        b"<pmd version=\"6.15.0\"/>"
    );
}

#[test]
fn report_symlink_and_oversized_report_are_incomplete() {
    let dir = TestDir::new();
    let outside = TestDir::new();
    let target = outside.0.join("outside.xml");
    fs::write(&target, b"outside").expect("外部文件");
    let slot = prepare_fresh_report(&dir.0, "pmd.xml", 5).expect("准备报告槽");
    symlink(&target, dir.0.join("pmd.xml")).expect("报告符号链接");
    assert!(slot.read().is_err());
    fs::remove_file(dir.0.join("pmd.xml")).expect("清除测试链接");
    fs::write(dir.0.join("pmd.xml"), b"too-large").expect("超大报告");
    assert_eq!(
        slot.read().expect_err("超大报告").kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(fs::read(target).expect("外部文件不变"), b"outside");
}

#[test]
fn report_hard_link_to_an_external_file_is_rejected() {
    let dir = TestDir::new();
    let outside = TestDir::new();
    let target = outside.0.join("outside.xml");
    fs::write(&target, b"external report").expect("外部报告");
    let slot = prepare_fresh_report(&dir.0, "pmd.xml", 1024).expect("准备报告槽");
    fs::hard_link(&target, dir.0.join("pmd.xml")).expect("建立硬链接");
    assert_eq!(
        slot.read().expect_err("硬链接不能成为本轮报告").kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(fs::read(target).expect("外部报告保留"), b"external report");
}

#[test]
fn unsafe_report_roots_and_names_are_rejected_before_execution() {
    let dir = TestDir::new();
    let alias = dir.0.join("alias");
    symlink(&dir.0, &alias).expect("目录链接");
    assert!(prepare_fresh_report(&alias, "pmd.xml", 1024).is_err());
    for name in ["../pmd.xml", "nested/pmd.xml", "", ".", ".."] {
        assert!(prepare_fresh_report(&dir.0, name, 1024).is_err());
    }
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).expect("公开目录");
    assert!(prepare_fresh_report(&dir.0, "pmd.xml", 1024).is_err());
}

fn native_spec(dir: &TestDir, script: &str) -> ProcessSpec {
    ProcessSpec {
        executable: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        cwd: dir.0.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(3),
        output_limit_bytes: 1024,
    }
}

#[test]
fn stale_report_blocks_native_spawn() {
    let dir = TestDir::new();
    fs::write(dir.0.join("pmd.xml"), b"old").expect("旧报告");
    let spec = native_spec(&dir, "touch spawned.marker");
    let failure = run_process_recorded_with_report(
        &spec,
        &AtomicBool::new(false),
        &dir.0,
        "run.log",
        &dir.0,
        "pmd.xml",
        1024,
    )
    .expect_err("旧报告不得触发执行");
    assert_eq!(failure.kind, ReportEvidenceFailureKind::Preflight);
    assert!(failure.outcome.is_none());
    assert!(!dir.0.join("spawned.marker").exists());
}

#[test]
fn native_exit_zero_without_a_new_report_is_incomplete() {
    let dir = TestDir::new();
    let spec = native_spec(&dir, "exit 0");
    let failure = run_process_recorded_with_report(
        &spec,
        &AtomicBool::new(false),
        &dir.0,
        "run.log",
        &dir.0,
        "pmd.xml",
        1024,
    )
    .expect_err("缺报告不能完成");
    assert_eq!(failure.kind, ReportEvidenceFailureKind::ReportRead);
    assert_eq!(
        failure.outcome.as_ref().map(|value| value.termination),
        Some(Termination::Exited(0))
    );
    assert!(dir.0.join("run.log").exists());
}

#[test]
fn native_process_report_and_private_log_share_one_run() {
    let dir = TestDir::new();
    let spec = native_spec(&dir, "printf '<pmd/>' > pmd.xml");
    let result = run_process_recorded_with_report(
        &spec,
        &AtomicBool::new(false),
        &dir.0,
        "run.log",
        &dir.0,
        "pmd.xml",
        1024,
    )
    .expect("本轮进程与报告均有证据");
    assert_eq!(result.outcome.termination, Termination::Exited(0));
    assert_eq!(result.report, b"<pmd/>");
    assert!(dir.0.join("run.log").exists());
}

#[test]
fn failing_native_process_keeps_new_report_for_partial_diagnostics() {
    let dir = TestDir::new();
    let spec = native_spec(&dir, "printf '<pmd/>' > pmd.xml; exit 7");
    let result = run_process_recorded_with_report(
        &spec,
        &AtomicBool::new(false),
        &dir.0,
        "run.log",
        &dir.0,
        "pmd.xml",
        1024,
    )
    .expect("部分报告应保留并由适配器判未完成");
    assert_eq!(result.outcome.termination, Termination::Exited(7));
    assert_eq!(result.report, b"<pmd/>");
}

#[test]
fn private_log_failure_retains_native_outcome_and_never_returns_report_success() {
    let dir = TestDir::new();
    fs::write(dir.0.join("run.log"), b"old").expect("旧日志");
    let spec = native_spec(&dir, "printf '<pmd/>' > pmd.xml");
    let failure = run_process_recorded_with_report(
        &spec,
        &AtomicBool::new(false),
        &dir.0,
        "run.log",
        &dir.0,
        "pmd.xml",
        1024,
    )
    .expect_err("日志未持久化不能签发完整报告");
    assert_eq!(failure.kind, ReportEvidenceFailureKind::ProcessEvidence);
    assert_eq!(
        failure.outcome.as_ref().map(|value| value.termination),
        Some(Termination::Exited(0))
    );
    assert_eq!(fs::read(dir.0.join("run.log")).expect("旧日志保留"), b"old");
}
