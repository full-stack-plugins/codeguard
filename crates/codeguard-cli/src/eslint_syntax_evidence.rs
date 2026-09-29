//! ESLint 原生确认任务中的脱敏 WASM 疑似位置；只具待核实证据权威。

use codeguard_adapters::bundled_grammar_candidates;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// 从当轮候选反馈提取有界坐标，不保存源码片段或语法节点文本。
/// 参数为初检反馈、工作区相对路径和当前源码字节；返回绑定固定 grammar 的疑似证据。
pub(crate) fn project(precheck: &Value, scope: &str, source: &[u8]) -> Result<Value, &'static str> {
    let observations = precheck["observations"]
        .as_array()
        .filter(|rows| rows.len() <= 128)
        .ok_or("syntax_evidence_observations_invalid")?;
    let positions: Vec<Value> = observations
        .iter()
        .map(|row| {
            json!({
                "classification":row["classification"],
                "kind":row["kind"],
                "start_line":row["start_line"],
                "start_column":row["start_column"],
                "end_line":row["end_line"],
                "end_column":row["end_column"],
                "group_id":row["group_id"]
            })
        })
        .collect();
    let evidence = json!({
        "schema_version":"1.0.0",
        "backend":"bundled_tree_sitter_wasm_candidate",
        "language":precheck["language"],
        "scope":scope,
        "source_sha256":precheck["source_sha256"],
        "grammar_sha256":precheck["grammar_sha256"],
        "grammar_qualified":precheck["grammar_qualified"],
        "precheck_reason":precheck["reason"],
        "truncated":precheck["truncated"],
        "observations":positions
    });
    if !valid(&evidence, scope, source) {
        return Err("syntax_evidence_identity_or_position_invalid");
    }
    Ok(evidence)
}

/// 核对当前源码、固定 grammar 身份和每个 Unicode 标量坐标。
/// 参数为不可信报告投影、工作区相对路径及当前源码；返回能否作为未核实任务证据。
pub(crate) fn valid(evidence: &Value, scope: &str, source: &[u8]) -> bool {
    let keys = [
        "schema_version",
        "backend",
        "language",
        "scope",
        "source_sha256",
        "grammar_sha256",
        "grammar_qualified",
        "precheck_reason",
        "truncated",
        "observations",
    ];
    let Some(rows) = evidence["observations"]
        .as_array()
        .filter(|rows| rows.len() <= 128)
    else {
        return false;
    };
    let Some(language) = evidence["language"].as_str() else {
        return false;
    };
    if !evidence.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) || evidence["schema_version"] != "1.0.0"
        || evidence["backend"] != "bundled_tree_sitter_wasm_candidate"
        || evidence["scope"] != scope
        || evidence["source_sha256"] != format!("{:x}", Sha256::digest(source))
        || evidence["grammar_qualified"] != false
        || !matches_language(scope, language)
        || !evidence["truncated"].is_boolean()
        || (evidence["truncated"] == true
            && evidence["precheck_reason"] != "recovery_budget_truncated")
        || (evidence["truncated"] == false
            && evidence["precheck_reason"] != "grammar_version_unqualified")
    {
        return false;
    }
    let Ok(manifest) = bundled_grammar_candidates() else {
        return false;
    };
    if !manifest.assets.iter().any(|asset| {
        asset.language == language
            && asset.sha256 == evidence["grammar_sha256"]
            && asset.release_status == "candidate_unvalidated"
    }) {
        return false;
    }
    let Ok(text) = std::str::from_utf8(source) else {
        return false;
    };
    let lines: Vec<&str> = text.split('\n').collect();
    rows.iter().all(|row| valid_position(row, &lines))
}

fn matches_language(scope: &str, language: &str) -> bool {
    match language {
        "typescript" => [".ts", ".mts", ".cts"]
            .iter()
            .any(|suffix| scope.ends_with(suffix)),
        "tsx" => scope.ends_with(".tsx"),
        _ => false,
    }
}

fn valid_position(row: &Value, lines: &[&str]) -> bool {
    let keys = [
        "classification",
        "kind",
        "start_line",
        "start_column",
        "end_line",
        "end_column",
        "group_id",
    ];
    let (Some(start_line), Some(start_column), Some(end_line), Some(end_column), Some(group_id)) = (
        row["start_line"].as_u64(),
        row["start_column"].as_u64(),
        row["end_line"].as_u64(),
        row["end_column"].as_u64(),
        row["group_id"].as_u64(),
    ) else {
        return false;
    };
    row.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) && row["classification"] == "suspected"
        && matches!(row["kind"].as_str(), Some("ERROR" | "MISSING"))
        && group_id > 0
        && start_line > 0
        && end_line >= start_line
        && end_line <= lines.len() as u64
        && start_column > 0
        && end_column > 0
        && start_column <= lines[start_line as usize - 1].chars().count() as u64 + 1
        && end_column <= lines[end_line as usize - 1].chars().count() as u64 + 1
        && (start_line, start_column) <= (end_line, end_column)
}
