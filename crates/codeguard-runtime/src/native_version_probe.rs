//! 共享的有界原生版本探测，不解析源码质量或批准来源。
use crate::{
    NativeVersionObservation, NativeVersionRequest, Termination, read_bounded_regular_file,
    run_process_recorded,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 使用共同截止时间核对工具身份、精确版本和私有日志。
/// 参数由适配器冻结；结果只供环境诊断，不能作为质量门禁通过。
#[must_use]
pub fn observe_native_version(
    request: &NativeVersionRequest,
    cancelled: &AtomicBool,
) -> NativeVersionObservation {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        return incomplete("request_cancelled", Some(Termination::Cancelled));
    }
    if request.process.deadline <= Instant::now() {
        return incomplete("deadline_exhausted", Some(Termination::DeadlineBeforeStart));
    }
    if request.expected_stdout.is_empty()
        || request.expected_stdout.len() > 4096
        || request.process.stdin.is_some()
        || request.process.args.len() != 1
        || !matches!(
            request.process.args[0].to_str(),
            Some("--version" | "-version" | "version" | "-V")
        )
    {
        return incomplete("version_spec_invalid", Some(Termination::InvalidSpec));
    }
    if tool_hash(&request.process.executable) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_mismatch", None);
    }
    let outcome = match run_process_recorded(
        &request.process,
        cancelled,
        &request.evidence_root,
        &request.log_name,
    ) {
        Ok(outcome) => outcome,
        Err(failure) => {
            return incomplete(failure.kind.reason(), Some(failure.outcome.termination));
        }
    };
    let termination = Some(outcome.termination);
    if outcome.termination != Termination::Exited(0) {
        return incomplete(termination_reason(outcome.termination), termination);
    }
    if !outcome.stderr.is_empty() {
        return incomplete("version_stderr_unexpected", termination);
    }
    if outcome.stdout != request.expected_stdout {
        return incomplete("tool_version_mismatch", termination);
    }
    if tool_hash(&request.process.executable) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_changed", termination);
    }
    if Instant::now() >= request.process.deadline {
        return incomplete("request_deadline_exceeded", termination);
    }
    NativeVersionObservation {
        complete: true,
        reason: None,
        termination,
    }
}

fn tool_hash(path: &std::path::Path) -> Option<[u8; 32]> {
    let bytes = read_bounded_regular_file(path, 128 * 1024 * 1024).ok()?;
    ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .try_into()
        .ok()
}
fn incomplete(reason: &'static str, termination: Option<Termination>) -> NativeVersionObservation {
    NativeVersionObservation {
        complete: false,
        reason: Some(reason),
        termination,
    }
}
fn termination_reason(termination: Termination) -> &'static str {
    match termination {
        Termination::Exited(_) => "version_native_exit_nonzero",
        Termination::Signaled => "version_signaled",
        Termination::InvalidSpec => "version_spec_invalid",
        Termination::DeadlineBeforeStart => "deadline_exhausted",
        Termination::Cancelled => "request_cancelled",
        Termination::TimedOut => "version_timed_out",
        Termination::OutputLimit => "version_output_limit",
        Termination::SpawnFailure => "version_spawn_failed",
        Termination::ReadFailure => "version_pipe_read_failed",
        Termination::WriteFailure => "version_stdin_write_failed",
        Termination::CleanupFailure => "version_cleanup_failed",
        Termination::UnsupportedPlatform => "version_platform_unsupported",
    }
}
