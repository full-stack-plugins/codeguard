use crate::checkstyle_probe_request::CheckstyleProbeRequest;
use crate::checkstyle_probe_result::CheckstyleProbeResult;
use codeguard_adapters::{
    CheckstyleCommand, CheckstyleParsed, checkstyle_native_failure_reason,
    evaluate_checkstyle_report,
};
use codeguard_runtime::{
    ProcessSpec, ReportEvidenceFailureKind, Termination, read_bounded_regular_file,
    run_process_recorded_with_report,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 以统一 runtime 执行显式工具，并核对前后四个输入及本轮私有 XML。
/// 参数包含独立冻结摘要与共同预算；返回局部证据，不安装、联网下载或授予门禁权威。
#[must_use]
pub fn run_checkstyle_probe(
    req: &CheckstyleProbeRequest,
    cancelled: &AtomicBool,
) -> CheckstyleProbeResult {
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason, None);
    }
    if !cfg!(unix)
        || req.expected_version != "10.21.4"
        || req.run_id.is_empty()
        || req.run_id.len() > 64
        || !req
            .run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return incomplete("invalid_checkstyle_probe_request", None);
    }
    let inputs = [
        (&req.source, 16 * 1024 * 1024),
        (&req.config, 1024 * 1024),
        (&req.java, 128 * 1024 * 1024),
        (&req.jar, 128 * 1024 * 1024),
    ];
    if inputs
        .iter()
        .enumerate()
        .any(|(i, (path, _))| inputs[i + 1..].iter().any(|(other, _)| path == other))
        || req.expected_sha256.len() != 4
        || inputs
            .iter()
            .any(|(p, _)| !req.expected_sha256.contains_key(*p))
    {
        return incomplete("checkstyle_input_identity_missing", None);
    }
    let report_name = format!("{}-checkstyle.xml", req.run_id);
    let command = CheckstyleCommand {
        source: req.source.clone(),
        config: req.config.clone(),
        jar: req.jar.clone(),
        report: req.report_dir.join(&report_name),
    };
    let Ok(args) = command.args() else {
        return incomplete("invalid_checkstyle_command", None);
    };
    for (path, limit) in inputs {
        if let Some(reason) = stopped(req, cancelled) {
            return incomplete(reason, None);
        }
        if std::fs::canonicalize(path).ok().as_ref() != Some(path)
            || !identity_matches(req, path, limit)
        {
            return incomplete("checkstyle_input_identity_mismatch", None);
        }
    }
    let Some(cwd) = req.source.parent() else {
        return incomplete("invalid_checkstyle_command", None);
    };
    let spec = ProcessSpec {
        executable: req.java.clone(),
        args,
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: req.deadline,
        output_limit_bytes: 1024 * 1024,
    };
    let execution = match run_process_recorded_with_report(
        &spec,
        cancelled,
        &req.evidence_dir,
        &format!("{}-checkstyle.log", req.run_id),
        &req.report_dir,
        &report_name,
        16 * 1024 * 1024,
    ) {
        Ok(execution) => execution,
        Err(failure) => {
            if failure.kind == ReportEvidenceFailureKind::ReportRead {
                if let Some(outcome) = &failure.outcome {
                    let exit = match outcome.termination {
                        Termination::Exited(code) => Some(code),
                        _ => None,
                    };
                    if let Some(reason) = checkstyle_native_failure_reason(exit, &outcome.stderr) {
                        for (path, limit) in inputs {
                            if let Some(stopped_reason) = stopped(req, cancelled) {
                                return incomplete(stopped_reason, None);
                            }
                            if std::fs::canonicalize(path).ok().as_ref() != Some(path)
                                || !identity_matches(req, path, limit)
                            {
                                return incomplete("checkstyle_input_changed", None);
                            }
                        }
                        if let Some(stopped_reason) = stopped(req, cancelled) {
                            return incomplete(stopped_reason, None);
                        }
                        return incomplete(reason, None);
                    }
                }
            }
            return incomplete(
                match failure.kind {
                    ReportEvidenceFailureKind::Preflight => "checkstyle_report_preflight_failed",
                    ReportEvidenceFailureKind::ProcessEvidence => {
                        "checkstyle_process_evidence_failed"
                    }
                    ReportEvidenceFailureKind::ReportRead => "checkstyle_report_read_failed",
                    ReportEvidenceFailureKind::DeadlineExceeded => "request_deadline_exceeded",
                },
                None,
            );
        }
    };
    let exit = match execution.outcome.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    };
    let result = evaluate_checkstyle_report(
        &execution.report,
        &req.expected_version,
        &[req.source.to_string_lossy().into_owned()],
        exit,
    );
    for (path, limit) in inputs {
        if let Some(reason) = stopped(req, cancelled) {
            return incomplete(reason, Some(result.parsed));
        }
        if std::fs::canonicalize(path).ok().as_ref() != Some(path)
            || !identity_matches(req, path, limit)
        {
            return incomplete("checkstyle_input_changed", Some(result.parsed));
        }
    }
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason, Some(result.parsed));
    }
    CheckstyleProbeResult {
        local_coherent: result.local_coherent,
        reason: if result.local_coherent {
            result.reason
        } else {
            checkstyle_native_failure_reason(exit, &execution.outcome.stderr).or(result.reason)
        },
        parsed: Some(result.parsed),
    }
}

fn stopped(req: &CheckstyleProbeRequest, cancelled: &AtomicBool) -> Option<&'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Some("request_cancelled")
    } else if Instant::now() >= req.deadline {
        Some("request_deadline_exceeded")
    } else {
        None
    }
}

fn identity_matches(req: &CheckstyleProbeRequest, path: &std::path::Path, limit: u64) -> bool {
    let Ok(bytes) = read_bounded_regular_file(path, limit) else {
        return false;
    };
    req.expected_sha256.get(path) == Some(&<[u8; 32]>::from(Sha256::digest(bytes)))
}

fn incomplete(reason: &'static str, parsed: Option<CheckstyleParsed>) -> CheckstyleProbeResult {
    CheckstyleProbeResult {
        local_coherent: false,
        reason: Some(reason),
        parsed,
    }
}
