//! 通过统一 runtime 执行显式 ESLint，所有观察仍无项目/策略批准权威。
use crate::{eslint_probe_request::EslintProbeRequest, eslint_probe_result::EslintProbeResult};
use codeguard_adapters::{EslintParsed, eslint_report_version_matches, parse_eslint_json};
use codeguard_runtime::{
    ProcessSpec, ReportEvidenceFailureKind, Termination, prepare_fresh_report,
    read_bounded_regular_file, run_process_recorded, run_process_recorded_with_report,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 按显式工具/原配置/冻结源集执行版本和扫描，保留本轮有界 JSON。
/// 返回局部观察；不解析 JS 配置归属、下载工具、批准插件或签发完整项目门禁。
#[must_use]
pub fn run_eslint_probe(req: &EslintProbeRequest, cancelled: &AtomicBool) -> EslintProbeResult {
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason, None);
    }
    if !eslint_report_version_matches(&req.expected_version, &req.expected_version)
        || req.run_id.is_empty()
        || req.run_id.len() > 64
        || !req
            .run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || std::fs::canonicalize(&req.cwd).ok().as_ref() != Some(&req.cwd)
        || !req.cwd.is_dir()
    {
        return incomplete("eslint_probe_request_invalid", None);
    }
    let Ok(args) = req.command.args() else {
        return incomplete("eslint_command_invalid", None);
    };
    let mut inputs = vec![
        (&req.command.node, 128 * 1024 * 1024),
        (&req.command.entry, 16 * 1024 * 1024),
        (&req.command.config, 1024 * 1024),
    ];
    inputs.extend(req.command.sources.iter().map(|p| (p, 16 * 1024 * 1024)));
    // 与请求构造方镜像：TS 方言且包根存在 tsconfig.json 时冻结为输入身份。
    // 任何存在形态都纳入核对，中途换成链接或新建都能被识别为输入变化而非静默放行。
    let mut tsconfig_input: Option<PathBuf> = None;
    if req.command.sources.iter().any(|p| {
        p.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "ts" | "tsx" | "mts" | "cts"))
    }) {
        let tsconfig = req.cwd.join("tsconfig.json");
        if std::fs::symlink_metadata(&tsconfig).is_ok()
            && !inputs.iter().any(|(path, _)| **path == tsconfig)
        {
            tsconfig_input = Some(tsconfig);
        }
    }
    if let Some(tsconfig) = &tsconfig_input {
        inputs.push((tsconfig, 1024 * 1024));
    }
    if req.expected_sha256.len()
        != inputs
            .iter()
            .map(|(p, _)| *p)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
        || inputs
            .iter()
            .any(|(p, _)| !req.expected_sha256.contains_key(*p))
    {
        return incomplete("eslint_input_identity_missing", None);
    }
    if let Some(reason) = check_inputs(req, cancelled, &inputs, "eslint_input_identity_mismatch") {
        return incomplete(reason, None);
    }
    let report_name = format!("{}.json", req.run_id);
    let Some(report_dir) = req.command.report.parent() else {
        return incomplete("eslint_report_path_invalid", None);
    };
    if req.command.report.file_name().and_then(|n| n.to_str()) != Some(&report_name)
        || prepare_fresh_report(report_dir, &report_name, 16 * 1024 * 1024).is_err()
    {
        return incomplete("eslint_report_preflight_failed", None);
    }
    let version_spec = ProcessSpec {
        executable: req.command.node.clone(),
        args: vec![
            "--".into(),
            req.command.entry.as_os_str().to_owned(),
            "--version".into(),
        ],
        cwd: req.cwd.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: req.deadline,
        output_limit_bytes: 64 * 1024,
    };
    let version = match run_process_recorded(
        &version_spec,
        cancelled,
        &req.evidence_dir,
        &format!("{}-version.log", req.run_id),
    ) {
        Ok(v) => v,
        Err(failure) => {
            let reason = stopped(req, cancelled)
                .or_else(|| interruption_reason(failure.outcome.termination))
                .unwrap_or("eslint_version_evidence_failed");
            return incomplete(reason, None);
        }
    };
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason, None);
    }
    if version.termination != Termination::Exited(0) || !version.stderr.is_empty() {
        return incomplete("eslint_version_execution_incomplete", None);
    }
    if version.stdout != format!("v{}\n", req.expected_version).as_bytes() {
        return incomplete("eslint_version_mismatch", None);
    }
    if let Some(reason) = check_inputs(req, cancelled, &inputs, "eslint_input_changed") {
        return incomplete(reason, None);
    }
    let spec = ProcessSpec {
        executable: req.command.node.clone(),
        args,
        cwd: req.cwd.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: req.deadline,
        output_limit_bytes: 1024 * 1024,
    };
    let execution = match run_process_recorded_with_report(
        &spec,
        cancelled,
        &req.evidence_dir,
        &format!("{}-eslint.log", req.run_id),
        report_dir,
        &report_name,
        16 * 1024 * 1024,
    ) {
        Ok(value) => value,
        Err(failure) => {
            // 中断原因优先于报告缺失，避免指引智能体修复无关报告或源码。
            let reason = stopped(req, cancelled)
                .or_else(|| {
                    failure
                        .outcome
                        .as_ref()
                        .and_then(|value| interruption_reason(value.termination))
                })
                .unwrap_or(match failure.kind {
                    ReportEvidenceFailureKind::Preflight => "eslint_report_preflight_failed",
                    ReportEvidenceFailureKind::ProcessEvidence => {
                        "eslint_execution_evidence_failed"
                    }
                    ReportEvidenceFailureKind::ReportRead => "eslint_report_read_failed",
                    ReportEvidenceFailureKind::DeadlineExceeded => "request_deadline_exceeded",
                });
            return incomplete(reason, None);
        }
    };
    let exit = match execution.outcome.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    };
    let files: Vec<_> = req
        .command
        .sources
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let parsed = parse_eslint_json(
        &execution.report,
        &req.expected_version,
        &req.expected_version,
        &files,
        exit,
        req.command.max_warnings.map(u64::from),
    );
    if let Some(reason) = check_inputs(req, cancelled, &inputs, "eslint_input_changed") {
        return incomplete(reason, Some(parsed));
    }
    EslintProbeResult {
        local_coherent: parsed.local_coherent,
        reason: parsed.reason,
        parsed: Some(parsed),
    }
}
fn check_inputs(
    req: &EslintProbeRequest,
    cancelled: &AtomicBool,
    inputs: &[(&PathBuf, u64)],
    failure: &'static str,
) -> Option<&'static str> {
    for (path, limit) in inputs {
        if let Some(reason) = stopped(req, cancelled) {
            return Some(reason);
        }
        if std::fs::canonicalize(path).ok().as_ref() != Some(*path)
            || read_bounded_regular_file(path, *limit)
                .ok()
                .is_none_or(|b| {
                    req.expected_sha256.get(*path) != Some(&<[u8; 32]>::from(Sha256::digest(b)))
                })
        {
            return Some(failure);
        }
    }
    stopped(req, cancelled)
}
fn stopped(req: &EslintProbeRequest, cancelled: &AtomicBool) -> Option<&'static str> {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        Some("request_cancelled")
    } else if Instant::now() >= req.deadline {
        Some("request_deadline_exceeded")
    } else {
        None
    }
}
fn incomplete(reason: &'static str, parsed: Option<EslintParsed>) -> EslintProbeResult {
    EslintProbeResult {
        local_coherent: false,
        reason: Some(reason),
        parsed,
    }
}

fn interruption_reason(termination: Termination) -> Option<&'static str> {
    match termination {
        Termination::Cancelled => Some("request_cancelled"),
        Termination::TimedOut | Termination::DeadlineBeforeStart => {
            Some("request_deadline_exceeded")
        }
        _ => None,
    }
}
