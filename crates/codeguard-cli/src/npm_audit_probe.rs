//! 统一runtime下的npm输入绑定观察；不执行清单脚本、不签发门禁。
use crate::{
    npm_audit_probe_request::NpmAuditProbeRequest, npm_audit_probe_result::NpmAuditProbeResult,
};
use codeguard_adapters::{NpmLockedNode, parse_npm_audit_json};
use codeguard_runtime::{
    ProcessEvidenceFailureKind, ProcessSpec, Termination, read_bounded_regular_file,
    run_process_recorded,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 使用冻结的原目录输入运行版本与审计，返回一致观察或具体未完成原因。
/// 尚未冻结Node/npm全部模块闭包、祖先配置、认证与系统网络边界。
#[must_use]
pub fn run_npm_audit_probe(
    req: &NpmAuditProbeRequest,
    cancelled: &AtomicBool,
) -> NpmAuditProbeResult {
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason);
    }
    let valid_version = semver_version(&req.expected_version);
    if !valid_version
        || req.run_id.is_empty()
        || req.run_id.len() > 64
        || !req
            .run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || std::fs::canonicalize(&req.cwd).ok().as_ref() != Some(&req.cwd)
        || !req.cwd.is_dir()
    {
        return incomplete("npm_probe_request_invalid");
    }
    let args = match &req.registry {
        Some(registry) => req.command.args_for_registry(registry),
        None => req.command.args(),
    };
    let Ok(args) = args else {
        return incomplete("npm_command_invalid");
    };
    let package = req.cwd.join("package.json");
    let lock = req.cwd.join("package-lock.json");
    let npmrc = req.cwd.join(".npmrc");
    let has_npmrc = match std::fs::symlink_metadata(&npmrc) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return incomplete("npm_project_config_unreadable"),
    };
    let mut inputs = vec![
        (&req.command.node, 128 * 1024 * 1024),
        (&req.command.entry, 16 * 1024 * 1024),
        (&package, 256 * 1024),
        (&lock, 8 * 1024 * 1024),
        (&req.command.user_config, 1024 * 1024),
        (&req.command.global_config, 1024 * 1024),
    ];
    if has_npmrc {
        inputs.push((&npmrc, 1024 * 1024));
    }
    let names: BTreeSet<_> = inputs.iter().map(|(p, _)| *p).collect();
    if names.len() != inputs.len()
        || req.expected_sha256.len() != inputs.len()
        || names.iter().any(|p| !req.expected_sha256.contains_key(*p))
    {
        return incomplete("npm_input_identity_missing");
    }
    let check = |reason| check_inputs(req, cancelled, &inputs, &npmrc, has_npmrc, reason, false);
    if let Some(reason) = check("npm_input_identity_mismatch") {
        return incomplete(reason);
    }
    let mut version_args = args.clone();
    version_args[2] = "--version".into();
    let version = run_process_recorded(
        &ProcessSpec {
            executable: req.command.node.clone(),
            args: version_args,
            cwd: req.cwd.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: req.deadline,
            output_limit_bytes: 64 * 1024,
        },
        cancelled,
        &req.evidence_dir,
        &format!("{}-version.log", req.run_id),
    );
    let Ok(version) = version else {
        return incomplete(stopped(req, cancelled).unwrap_or("npm_version_evidence_failed"));
    };
    if let Some(reason) = stopped(req, cancelled) {
        return incomplete(reason);
    }
    if version.termination != Termination::Exited(0)
        || !version.stderr.is_empty()
        || version.stdout != format!("{}\n", req.expected_version).as_bytes()
    {
        return incomplete("npm_version_mismatch");
    }
    if let Some(reason) = check("npm_input_changed") {
        return incomplete(reason);
    }
    let execution = run_process_recorded(
        &ProcessSpec {
            executable: req.command.node.clone(),
            args,
            cwd: req.cwd.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: req.deadline,
            output_limit_bytes: 8 * 1024 * 1024,
        },
        cancelled,
        &req.evidence_dir,
        &format!("{}-audit.log", req.run_id),
    );
    let (execution, evidence_deadline) = match execution {
        Ok(outcome) => (outcome, false),
        Err(failure) if failure.kind == ProcessEvidenceFailureKind::DeadlineExceeded => {
            (failure.outcome, true)
        }
        Err(_) => return incomplete("npm_audit_evidence_failed"),
    };
    if let Some(reason) = check_inputs(
        req,
        cancelled,
        &inputs,
        &npmrc,
        has_npmrc,
        "npm_input_changed",
        true,
    ) {
        return incomplete(reason);
    }
    let execution_reason = if execution.termination == Termination::Cancelled {
        return incomplete("cancelled");
    } else if evidence_deadline {
        Some("deadline")
    } else {
        match execution.termination {
            Termination::Exited(_) => None,
            Termination::Cancelled => return incomplete("cancelled"),
            Termination::TimedOut | Termination::DeadlineBeforeStart => Some("deadline"),
            Termination::OutputLimit => Some("npm_audit_output_limit"),
            _ => Some("npm_audit_execution_incomplete"),
        }
    };
    let exit = match execution.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    };
    let parsed = match parse_npm_audit_json(
        &execution.stdout,
        &req.expected_version,
        &req.expected_version,
        exit,
    ) {
        Ok(parsed) if execution_reason.is_none() => parsed,
        Ok(_) | Err("npm_audit_execution_incomplete")
            if execution_reason.is_some() || matches!(exit, Some(code) if code > 1) =>
        {
            // 未完成执行仅保留完整且含漏洞的报告；零发现不能充当干净证据。
            let Ok(parsed) = parse_npm_audit_json(
                &execution.stdout,
                &req.expected_version,
                &req.expected_version,
                Some(1),
            ) else {
                return incomplete(execution_reason.unwrap_or("npm_audit_execution_incomplete"));
            };
            if parsed.components.is_empty() {
                return incomplete(execution_reason.unwrap_or("npm_audit_execution_incomplete"));
            }
            parsed
        }
        Err(reason) => return incomplete(execution_reason.unwrap_or(reason)),
        Ok(_) => return incomplete("npm_audit_execution_incomplete"),
    };
    let lock_bytes = match read_bounded_regular_file(&lock, 8 * 1024 * 1024) {
        Ok(b) => b,
        Err(_) => return incomplete("npm_lock_unreadable"),
    };
    if req.expected_sha256.get(&lock) != Some(&<[u8; 32]>::from(Sha256::digest(&lock_bytes))) {
        return incomplete("npm_input_changed");
    }
    let nodes = match NpmLockedNode::parse(&lock_bytes) {
        Ok(n) => n,
        Err(reason) => return incomplete(reason),
    };
    if let Err(reason) = NpmLockedNode::bind(&nodes, &parsed) {
        return incomplete(reason);
    }
    if let Some(reason) = check_inputs(
        req,
        cancelled,
        &inputs,
        &npmrc,
        has_npmrc,
        "npm_input_changed",
        true,
    ) {
        return incomplete(reason);
    }
    let execution_reason =
        execution_reason.or_else(|| (Instant::now() >= req.deadline).then_some("deadline"));
    if execution_reason == Some("deadline") && parsed.components.is_empty() {
        return incomplete("deadline");
    }
    NpmAuditProbeResult {
        local_coherent: true,
        reason: execution_reason.or_else(|| {
            matches!(exit, Some(code) if code > 1).then_some("npm_audit_execution_incomplete")
        }),
        parsed: Some(parsed),
        locked_nodes: nodes,
    }
}
fn check_inputs(
    req: &NpmAuditProbeRequest,
    cancelled: &AtomicBool,
    inputs: &[(&PathBuf, u64)],
    npmrc: &PathBuf,
    has_npmrc: bool,
    reason: &'static str,
    allow_expired: bool,
) -> Option<&'static str> {
    for (path, limit) in inputs {
        if let Some(reason) = check_interruption(req, cancelled, allow_expired) {
            return Some(reason);
        }
        if std::fs::canonicalize(path).ok().as_ref() != Some(*path) {
            return Some(reason);
        }
        match read_bounded_regular_file(path, *limit) {
            Ok(bytes)
                if req.expected_sha256.get(*path)
                    == Some(&<[u8; 32]>::from(Sha256::digest(&bytes))) => {}
            _ => return Some(reason),
        }
    }
    if !has_npmrc
        && !matches!(std::fs::symlink_metadata(npmrc),Err(error) if error.kind()==std::io::ErrorKind::NotFound)
    {
        return Some(reason);
    }
    check_interruption(req, cancelled, allow_expired)
}
fn check_interruption(
    req: &NpmAuditProbeRequest,
    cancelled: &AtomicBool,
    allow_expired: bool,
) -> Option<&'static str> {
    if cancelled.load(Ordering::SeqCst) || codeguard_runtime::sigint_cancellation_requested() {
        Some("cancelled")
    } else if !allow_expired && Instant::now() >= req.deadline {
        Some("deadline")
    } else {
        None
    }
}
fn semver_version(version: &str) -> bool {
    // 不引入CLI自身的版本解析依赖；原生JSON解析器另按同一具体版本复核。
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts[0] == "11"
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
                && p.parse::<u64>().is_ok()
        })
}
fn stopped(req: &NpmAuditProbeRequest, cancelled: &AtomicBool) -> Option<&'static str> {
    if cancelled.load(Ordering::SeqCst) || codeguard_runtime::sigint_cancellation_requested() {
        Some("cancelled")
    } else if Instant::now() >= req.deadline {
        Some("deadline")
    } else {
        None
    }
}
fn incomplete(reason: &'static str) -> NpmAuditProbeResult {
    NpmAuditProbeResult {
        local_coherent: false,
        reason: Some(reason),
        parsed: None,
        locked_nodes: Vec::new(),
    }
}
