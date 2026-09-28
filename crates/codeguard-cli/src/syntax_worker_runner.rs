//! 候选语法工作进程的父进程边界；失败只成为未完成，绝不成为源码违规。

use crate::syntax_worker_candidate_observation::SyntaxWorkerCandidateObservation;
use crate::syntax_worker_envelope::SyntaxWorkerEnvelope;
use codeguard_adapters::bundled_grammar_candidates;
use codeguard_core::{SyntaxFileObservation, SyntaxFileState, assess_syntax_precheck};
use codeguard_runtime::{ProcessSpec, Termination, run_process};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 启动同一二进制的私有解析进程并核验来源、字节身份、位置和状态。
/// 输入为绝对二进制路径、固定语种、相对源码路径、本轮源码字节、共同截止时间及取消标记。
/// 返回只具候选观察权威的状态；内存隔离和正式 lint 接线尚未验收。
pub fn run_syntax_worker_candidate(
    executable: &Path,
    language: &str,
    relative_path: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<SyntaxWorkerCandidateObservation, String> {
    if !executable.is_absolute() || source.len() > 1024 * 1024 {
        return Err("syntax_worker_input_invalid".into());
    }
    let source_text =
        std::str::from_utf8(source).map_err(|_| "syntax_worker_source_encoding_invalid")?;
    let manifest = bundled_grammar_candidates()?;
    let asset = manifest
        .assets
        .iter()
        .find(|asset| asset.language == language)
        .ok_or("syntax_worker_language_unsupported")?;
    let expected_sha = format!("{:x}", Sha256::digest(source));
    let spec = ProcessSpec {
        executable: executable.to_path_buf(),
        args: vec![OsString::from("__syntax-worker"), OsString::from(language)],
        cwd: PathBuf::from("/"),
        env: BTreeMap::new(),
        stdin: Some(source.to_vec()),
        deadline,
        output_limit_bytes: 64 * 1024,
    };
    let process = run_process(&spec, cancelled);
    if process.termination != Termination::Exited(0) {
        return Err(format!(
            "syntax_worker_termination:{:?}",
            process.termination
        ));
    }
    let value = codeguard_adapters::parse_unique_json(&process.stdout).map_err(str::to_owned)?;
    let report: SyntaxWorkerEnvelope =
        serde_json::from_value(value).map_err(|_| "syntax_worker_report_invalid")?;
    if report.schema_version != "1.0.0"
        || report.report_type != "syntax_worker_candidate"
        || report.language != language
        || report.grammar_sha256 != asset.sha256
        || report.grammar_abi_version != asset.abi_version
        || report.source_sha256 != expected_sha
        || report.recoveries.len() > 128
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
        recoveries: report.recoveries,
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
