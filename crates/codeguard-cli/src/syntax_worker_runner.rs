//! 候选语法工作进程的父进程边界；失败只成为未完成，绝不成为源码违规。

use crate::syntax_worker_candidate_observation::SyntaxWorkerCandidateObservation;
use crate::syntax_worker_envelope::SyntaxWorkerEnvelope;
use codeguard_adapters::bundled_grammar_candidate;
use codeguard_core::{SyntaxFileObservation, SyntaxFileState, assess_syntax_precheck};
#[cfg(not(target_os = "linux"))]
use codeguard_runtime::run_process;
#[cfg(target_os = "linux")]
use codeguard_runtime::run_process_with_address_space_limit;
use codeguard_runtime::{ProcessSpec, Termination};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 启动同一二进制的私有解析进程并核验来源、字节身份、位置和状态。
/// 输入为绝对二进制路径、固定语种、相对源码路径、本轮源码字节、共同截止时间及取消标记。
/// 返回只具候选观察权威的状态；Linux 内存上限与正式检查覆盖仍待实测验收。
pub fn run_syntax_worker_candidate(
    executable: &Path,
    language: &str,
    relative_path: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<SyntaxWorkerCandidateObservation, String> {
    run_candidate(
        executable,
        language,
        relative_path,
        source,
        deadline,
        cancelled,
        false,
    )
}

/// 显式观察JavaScript直接绑定规则；暂仅供probe接线，其它消费者沿用旧协议。
/// 参数为当前二进制、语言、相对路径、冻结源码和统一预算；未知语言执行前拒绝。
pub fn run_syntax_worker_binding_candidate(
    executable: &Path,
    language: &str,
    relative_path: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<SyntaxWorkerCandidateObservation, String> {
    if language != "javascript" {
        return Err("syntax_binding_language_invalid".into());
    }
    run_candidate(
        executable,
        language,
        relative_path,
        source,
        deadline,
        cancelled,
        true,
    )
}

fn run_candidate(
    executable: &Path,
    language: &str,
    relative_path: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
    bindings: bool,
) -> Result<SyntaxWorkerCandidateObservation, String> {
    if !executable.is_absolute() || source.len() > 1024 * 1024 {
        return Err("syntax_worker_input_invalid".into());
    }
    let source_text =
        std::str::from_utf8(source).map_err(|_| "syntax_worker_source_encoding_invalid")?;
    let (asset, _) = bundled_grammar_candidate(language)
        .map_err(|reason| format!("syntax_worker_candidate_unavailable:{reason}"))?;
    let expected_sha = format!("{:x}", Sha256::digest(source));
    let mut args = vec![OsString::from("__syntax-worker"), OsString::from(language)];
    if bindings {
        args.push(OsString::from("--direct-bindings"));
    }
    let spec = ProcessSpec {
        executable: executable.to_path_buf(),
        args,
        cwd: PathBuf::from("/"),
        env: BTreeMap::new(),
        stdin: Some(source.to_vec()),
        deadline,
        output_limit_bytes: 64 * 1024,
    };
    // Linux 为整个私有进程设置地址空间硬上限；其它 Unix 平台尚无实测硬限制。
    #[cfg(target_os = "linux")]
    let process = run_process_with_address_space_limit(&spec, cancelled, 2 * 1024 * 1024 * 1024);
    #[cfg(not(target_os = "linux"))]
    let process = run_process(&spec, cancelled);
    if process.termination != Termination::Exited(0) {
        return Err(format!(
            "syntax_worker_termination:{:?}",
            process.termination
        ));
    }
    let value = codeguard_adapters::parse_unique_json(&process.stdout).map_err(str::to_owned)?;
    if value["schema_version"] == "1.0.0" && value.get("structural_observations").is_some() {
        return Err("syntax_worker_version_fields_mismatch".into());
    }
    if !matches!(value["schema_version"].as_str(), Some("1.4.0" | "1.5.0"))
        && value.get("parser_error_location_unavailable").is_some()
    {
        return Err("syntax_worker_version_fields_mismatch".into());
    }
    let report: SyntaxWorkerEnvelope =
        serde_json::from_value(value).map_err(|_| "syntax_worker_report_invalid")?;
    if !matches!(
        report.schema_version.as_str(),
        "1.0.0" | "1.1.0" | "1.2.0" | "1.3.0" | "1.4.0" | "1.5.0"
    ) || (report.schema_version == "1.0.0" && !report.structural_observations.is_empty())
        || (report.schema_version == "1.1.0"
            && (language != "python" || report.structural_observations.is_empty()))
        || (report.schema_version == "1.2.0"
            && (language != "go" || report.structural_observations.len() != 1))
        || (report.schema_version == "1.3.0"
            && (language != "cfquery" || report.structural_observations.is_empty()))
        || (report.schema_version == "1.4.0"
            && (report.parser_error_location_unavailable != Some(true) || !report.truncated))
        || (report.schema_version == "1.5.0"
            && (!bindings || language != "javascript" || report.structural_observations.is_empty()))
        || (language == "javascript"
            && !report.structural_observations.is_empty()
            && report.schema_version != "1.5.0")
        || (report.schema_version == "1.5.0"
            && report
                .parser_error_location_unavailable
                .is_some_and(|flag| !flag || !report.truncated))
        || report.report_type != "syntax_worker_candidate"
        || report.language != language
        || report.grammar_sha256 != asset.sha256
        || report.grammar_abi_version != asset.abi_version
        || report.source_sha256 != expected_sha
        || report.recoveries.len() > 128
        || report.recoveries.len() + report.structural_observations.len() > 128
        || report
            .structural_observations
            .iter()
            .any(|row| !row.valid(language, source))
    {
        return Err("syntax_worker_identity_mismatch".into());
    }
    for recovery in &report.recoveries {
        if !matches!(recovery.kind.as_str(), "ERROR" | "MISSING")
            || recovery.group_id == 0
            || recovery.syntax_kind.is_empty()
            || recovery.syntax_kind.len() > 128
            || recovery.syntax_kind.chars().any(char::is_control)
            || recovery.start_byte > recovery.end_byte
            || recovery.end_byte > source.len()
            || !source_text.is_char_boundary(recovery.start_byte)
            || !source_text.is_char_boundary(recovery.end_byte)
            || position(source, recovery.start_byte)
                != Some((recovery.start_row, recovery.start_column_byte))
            || position(source, recovery.end_byte)
                != Some((recovery.end_row, recovery.end_column_byte))
        {
            return Err("syntax_worker_recovery_invalid".into());
        }
    }
    let precheck = assess_syntax_precheck(
        &[SyntaxFileObservation {
            path: relative_path.into(),
            state: SyntaxFileState::Checked {
                recoveries: report.recoveries.len(),
                grammar_qualified: false,
                truncated: report.truncated,
            },
        }],
        true,
        false,
    )
    .map_err(str::to_owned)?;
    Ok(SyntaxWorkerCandidateObservation {
        source_sha256: expected_sha,
        grammar_sha256: asset.sha256.clone(),
        grammar_qualified: false,
        parser_error_location_unavailable: report
            .parser_error_location_unavailable
            .unwrap_or(false),
        recoveries: report.recoveries,
        structural_observations: report.structural_observations,
        precheck,
    })
}

fn position(source: &[u8], offset: usize) -> Option<(usize, usize)> {
    let preceding = source.get(..offset)?;
    let row = preceding.iter().filter(|byte| **byte == b'\n').count();
    let column = preceding
        .rsplit(|byte| *byte == b'\n')
        .next()
        .map_or(0, <[u8]>::len);
    Some((row, column))
}
