//! 候选 WASM 初检报告的严格读者；不授予原生 lint 或交付权威。

use codeguard_core::{
    SyntaxFileObservation, SyntaxFileState, SyntaxPrecheckOutcome, assess_syntax_precheck,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateReport {
    schema_version: String,
    report_type: String,
    backend: CandidateBackend,
    scope: CandidateScope,
    files: Vec<CandidateFile>,
    precheck: SyntaxPrecheckOutcome,
    native: CandidateNative,
    delivery: CandidateDelivery,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateBackend {
    kind: String,
    language: String,
    dialect: String,
    grammar_sha256: String,
    grammar_abi_version: u32,
    grammar_release_status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateScope {
    enumeration_complete: bool,
    cancelled: bool,
    excluded_files: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateFile {
    path: String,
    source_sha256: String,
    state: SyntaxFileState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateNative {
    status: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateDelivery {
    decision: String,
    authority: String,
}

/// 严格读取候选初检观察，绑定当前源码字节与固定 grammar 身份，并重新计算状态。
/// 参数为报告字节及工作区相对路径到本轮源码字节的映射；返回无交付权威的初检状态。
pub fn parse_syntax_precheck_candidate_report(
    raw: &[u8],
    source_bytes: &BTreeMap<String, Vec<u8>>,
) -> Result<SyntaxPrecheckOutcome, String> {
    let value = crate::parse_unique_json(raw).map_err(str::to_owned)?;
    let report: CandidateReport =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    if report.schema_version != "1.0.0" || report.report_type != "syntax_precheck_candidate" {
        return Err("syntax_precheck_report_version_or_type_invalid".into());
    }
    if report.backend.kind != "bundled_tree_sitter_wasm"
        || report.backend.grammar_release_status != "candidate_unvalidated"
    {
        return Err("syntax_precheck_backend_invalid".into());
    }
    let manifest = crate::bundled_grammar_candidates()?;
    let asset = manifest
        .assets
        .iter()
        .find(|asset| asset.language == report.backend.language)
        .ok_or("syntax_precheck_grammar_not_pinned")?;
    if asset.dialect != report.backend.dialect
        || asset.sha256 != report.backend.grammar_sha256
        || asset.abi_version != report.backend.grammar_abi_version
        || asset.release_status != report.backend.grammar_release_status
    {
        return Err("syntax_precheck_grammar_identity_mismatch".into());
    }
    if report.native.status != "not_run"
        || report.native.reason.trim().is_empty()
        || report.delivery.decision != "not_evaluated"
        || report.delivery.authority != "local_unverified"
    {
        return Err("syntax_precheck_authority_invalid".into());
    }
    let mut observations = Vec::with_capacity(report.files.len());
    for file in report.files {
        let source = source_bytes
            .get(&file.path)
            .ok_or("syntax_precheck_source_missing")?;
        let actual_digest = format!("{:x}", Sha256::digest(source));
        if file.source_sha256 != actual_digest {
            return Err("syntax_precheck_source_changed".into());
        }
        if matches!(
            file.state,
            SyntaxFileState::Checked {
                grammar_qualified: true,
                ..
            }
        ) {
            return Err("syntax_precheck_candidate_grammar_unqualified".into());
        }
        observations.push(SyntaxFileObservation {
            path: file.path,
            state: file.state,
        });
    }
    let actual = assess_syntax_precheck(
        &observations,
        report.scope.enumeration_complete,
        report.scope.cancelled,
    )
    .map_err(str::to_owned)?;
    if actual != report.precheck || report.scope.excluded_files > 1_000_000 {
        return Err("syntax_precheck_aggregate_mismatch".into());
    }
    Ok(actual)
}
