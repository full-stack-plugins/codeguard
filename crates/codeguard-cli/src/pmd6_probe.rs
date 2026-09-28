//! PMD 6 单文件原生执行的局部验收；不能据此声明 P3C 规则已加载。

use codeguard_adapters::{Pmd6Command, PmdParseState, PmdParsed, parse_pmd_xml};
use codeguard_runtime::{
    ProcessSpec, ReportEvidenceFailureKind, Termination, read_bounded_regular_file,
    run_process_recorded_with_report,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 固定源文件、启动器、规则集与私有报告路径的 PMD 6 试运行请求。
pub struct Pmd6ProbeRequest {
    /// 待检的绝对 Java 源文件。
    pub source: PathBuf,
    /// 已批准 PMD 6 制品中的原生 run.sh 路径。
    pub launcher: PathBuf,
    /// 启动器预期 SHA-256；完整制品锁定仍由上层负责。
    pub expected_launcher_sha256: [u8; 32],
    /// 已锁定 PMD 发行包的绝对目录，包含启动器及其委托执行的制品。
    pub bundle_root: PathBuf,
    /// 整个发行包目录树的预期 SHA-256 小写十六进制摘要。
    pub expected_bundle_sha256: String,
    /// 本轮固定的单个原生规则集引用。
    pub ruleset: String,
    /// 调用者预先创建的私有报告目录。
    pub report_dir: PathBuf,
    /// 调用者预先创建的私有日志目录。
    pub evidence_dir: PathBuf,
    /// 本轮文件名前缀。
    pub run_id: String,
    /// 从锁定工具上下文得出的 PMD 版本预期。
    pub expected_pmd_version: String,
    /// 调用者传递的共同绝对截止时间。
    pub deadline: Instant,
}

/// PMD 原生退出、报告范围及私有证据的局部一致性状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pmd6ProbeState {
    /// 本地证据相互一致；尚未证明规则集身份和 P3C 能力。
    LocalReportCoherent,
    /// 必需证据缺失或矛盾，不得签发检查通过。
    Incomplete,
}

/// 保留有效诊断和稳定失败原因的局部执行结果。
pub struct Pmd6ProbeResult {
    /// 局部证据状态。
    pub state: Pmd6ProbeState,
    /// 未完成原因；局部一致时为空。
    pub reason: Option<&'static str>,
    /// 可解析的原生报告；其文字在公开前仍须脱敏。
    pub parsed: Option<PmdParsed>,
}

/// 通过统一运行时调用原生 PMD 启动器，并核对执行前后源码、启动器和 XML 范围。
#[must_use]
pub fn run_pmd6_probe(request: &Pmd6ProbeRequest, cancelled: &AtomicBool) -> Pmd6ProbeResult {
    if request.run_id.is_empty()
        || !request
            .run_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        || !request.source.is_absolute()
        || !request.launcher.is_absolute()
        || !request.bundle_root.is_absolute()
        || request.expected_pmd_version.is_empty()
    {
        return incomplete("invalid_probe_request", None);
    }
    let Ok(source_before) = read_bounded_regular_file(&request.source, 16 * 1024 * 1024) else {
        return incomplete("source_unavailable", None);
    };
    let source_digest: [u8; 32] = Sha256::digest(&source_before).into();
    let bundle_path = fs::canonicalize(&request.bundle_root).ok();
    let launcher_path = fs::canonicalize(&request.launcher).ok();
    if bundle_path.as_deref() != Some(request.bundle_root.as_path())
        || launcher_path.as_deref() != Some(request.launcher.as_path())
        || !request.launcher.starts_with(&request.bundle_root)
    {
        return incomplete("launcher_outside_bundle", None);
    }
    if bundle_digest(&request.bundle_root).as_deref()
        != Some(request.expected_bundle_sha256.as_str())
    {
        return incomplete("bundle_identity_mismatch", None);
    }
    if hash_launcher(&request.launcher) != Some(request.expected_launcher_sha256) {
        return incomplete("tool_identity_mismatch", None);
    }
    let report_name = format!("{}-pmd.xml", request.run_id);
    let report = request.report_dir.join(&report_name);
    let Ok(args) = (Pmd6Command {
        source: request.source.clone(),
        ruleset: request.ruleset.clone(),
        report,
    })
    .args() else {
        return incomplete("invalid_pmd_command", None);
    };
    let Some(cwd) = request.source.parent() else {
        return incomplete("source_unavailable", None);
    };
    let spec = ProcessSpec {
        executable: request.launcher.clone(),
        args,
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 16 * 1024 * 1024,
    };
    let log_name = format!("{}-pmd.log", request.run_id);
    let execution = match run_process_recorded_with_report(
        &spec,
        cancelled,
        &request.evidence_dir,
        &log_name,
        &request.report_dir,
        &report_name,
        16 * 1024 * 1024,
    ) {
        Ok(execution) => execution,
        Err(failure) => {
            let reason = match failure.kind {
                ReportEvidenceFailureKind::Preflight => "report_preflight_failed",
                ReportEvidenceFailureKind::ProcessEvidence => "process_evidence_failed",
                ReportEvidenceFailureKind::ReportRead => "report_read_failed",
                ReportEvidenceFailureKind::DeadlineExceeded => "request_deadline_exceeded",
            };
            return incomplete(reason, None);
        }
    };
    let parsed = parse_pmd_xml(&execution.report, &request.expected_pmd_version);
    if bundle_digest(&request.bundle_root).as_deref()
        != Some(request.expected_bundle_sha256.as_str())
    {
        return incomplete("bundle_identity_changed", Some(parsed));
    }
    if hash_launcher(&request.launcher) != Some(request.expected_launcher_sha256) {
        return incomplete("tool_identity_changed", Some(parsed));
    }
    let Ok(source_after) = read_bounded_regular_file(&request.source, 16 * 1024 * 1024) else {
        return incomplete("source_changed", Some(parsed));
    };
    if <[u8; 32]>::from(Sha256::digest(source_after)) != source_digest {
        return incomplete("source_changed", Some(parsed));
    }
    if parsed.state == PmdParseState::Incomplete {
        return incomplete(parsed.reason.unwrap_or("invalid_pmd_report"), Some(parsed));
    }
    if parsed.files.len() != 1 || parsed.files[0] != request.source.to_string_lossy() {
        return incomplete("report_source_mismatch", Some(parsed));
    }
    let expected_exit = if parsed.diagnostics.is_empty() { 0 } else { 4 };
    if execution.outcome.termination != Termination::Exited(expected_exit) {
        return incomplete("pmd_exit_report_conflict", Some(parsed));
    }
    if Instant::now() >= request.deadline {
        return incomplete("request_deadline_exceeded", Some(parsed));
    }
    Pmd6ProbeResult {
        state: Pmd6ProbeState::LocalReportCoherent,
        reason: None,
        parsed: Some(parsed),
    }
}

fn hash_launcher(path: &Path) -> Option<[u8; 32]> {
    let bytes = read_bounded_regular_file(path, 128 * 1024 * 1024).ok()?;
    Some(Sha256::digest(bytes).into())
}

fn bundle_digest(path: &Path) -> Option<String> {
    crate::tool_identity::hash_bundle_tree(path).ok()
}

fn incomplete(reason: &'static str, parsed: Option<PmdParsed>) -> Pmd6ProbeResult {
    Pmd6ProbeResult {
        state: Pmd6ProbeState::Incomplete,
        reason: Some(reason),
        parsed,
    }
}
