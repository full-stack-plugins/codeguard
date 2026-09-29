//! Ruff 缺失或项目未声明 Ruff 时的 Python WASM 语法候选观察；不改变原生报告或交付门禁。

use crate::syntax_worker_runner::run_syntax_worker_candidate;
use crate::typescript_syntax_precheck::map_observations;
use codeguard_adapters::bundled_grammar_candidates;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const MAX_FILES: usize = 8;
const MAX_OBSERVATIONS: usize = 128;

/// 仅为本轮缺少可运行 Ruff 上下文的文件补充有界疑似位置。
/// 参数是规范化项目根、原生反馈和整轮截止时间；返回无原生或交付权威的候选报告。
pub(crate) fn observe(root: &Path, feedback: &Value, deadline: Instant) -> Option<Value> {
    let candidates: Vec<&Value> = feedback["files"]
        .as_array()?
        .iter()
        .filter(|file| {
            file["run_status"] == "incomplete"
                && file["reason"].as_str().is_some_and(|reason| {
                    matches!(
                        reason,
                        "ruff_tool_not_found"
                            | "project_ruff_config_not_found"
                            | "ruff_section_not_declared"
                            | "ruff_lint_intent_not_explicit"
                            | "ruff_config_not_resolved"
                    )
                })
                && file["findings"].as_array().is_some_and(Vec::is_empty)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let asset = bundled_grammar_candidates().ok().and_then(|manifest| {
        manifest
            .assets
            .into_iter()
            .find(|asset| asset.language == "python")
    });
    let executable = std::env::current_exe().ok();
    let mut observations = Vec::new();
    let mut checked = Vec::new();
    let mut unavailable = Vec::new();
    let mut truncated = false;
    for (index, file) in candidates.iter().enumerate() {
        if index >= MAX_FILES {
            truncated = true;
            if unavailable.len() < MAX_FILES {
                unavailable.push(json!({"path":file["path"],"reason":"file_budget_exceeded"}));
            }
            continue;
        }
        let Some(path) = file["path"].as_str().filter(|path| safe_python_path(path)) else {
            unavailable.push(json!({"path":file["path"],"reason":"source_path_invalid"}));
            continue;
        };
        if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
            unavailable.push(json!({"path":path,"reason":"request_deadline_or_cancelled"}));
            continue;
        }
        let source_path = root.join(path);
        if !std::fs::symlink_metadata(&source_path).is_ok_and(|metadata| metadata.is_file())
            || source_path.canonicalize().ok().as_deref() != Some(source_path.as_path())
        {
            unavailable.push(json!({"path":path,"reason":"source_unavailable"}));
            continue;
        }
        let Ok(source) = read_bounded_regular_file(&source_path, 1024 * 1024) else {
            unavailable.push(json!({"path":path,"reason":"source_unavailable_or_over_budget"}));
            continue;
        };
        let source_sha = format!("{:x}", Sha256::digest(&source));
        if file["source_sha256"]
            .as_str()
            .is_some_and(|expected| expected != source_sha)
        {
            unavailable.push(json!({"path":path,"reason":"source_changed"}));
            continue;
        }
        let (Some(asset), Some(executable)) = (&asset, &executable) else {
            unavailable.push(json!({"path":path,"reason":"grammar_or_worker_unavailable"}));
            continue;
        };
        let Ok(result) = run_syntax_worker_candidate(
            executable,
            "python",
            path,
            &source,
            deadline,
            &AtomicBool::new(false),
        ) else {
            unavailable.push(json!({"path":path,"reason":"worker_incomplete"}));
            continue;
        };
        if result.grammar_sha256 != asset.sha256 {
            unavailable.push(json!({"path":path,"reason":"grammar_identity_changed"}));
            continue;
        }
        let Ok(mapped) = map_observations(&source, &result.recoveries) else {
            unavailable.push(json!({"path":path,"reason":"position_mapping_incomplete"}));
            continue;
        };
        checked.push(json!({"path":path,"source_sha256":source_sha,"grammar_sha256":asset.sha256}));
        truncated |= result.precheck.truncated_files > 0;
        for mut row in mapped {
            if observations.len() >= MAX_OBSERVATIONS {
                truncated = true;
                break;
            }
            row["path"] = json!(path);
            row["source_sha256"] = json!(source_sha);
            observations.push(row);
        }
    }
    let reason = if !unavailable.is_empty() {
        "partial_unavailable"
    } else if truncated {
        "recovery_or_file_budget_truncated"
    } else {
        "grammar_version_unqualified"
    };
    Some(json!({
        "backend":"bundled_tree_sitter_wasm_candidate",
        "language":"python",
        "status":"incomplete",
        "reason":reason,
        "grammar_sha256":asset.as_ref().map(|asset| asset.sha256.as_str()),
        "grammar_qualified":false,
        "selected_files":candidates.len(),
        "checked_files":checked.len(),
        "checked":checked,
        "unavailable":unavailable,
        "truncated":truncated,
        "observations":observations,
    }))
}

fn safe_python_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && value.len() <= 512
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && path
            .extension()
            .is_some_and(|ext| matches!(ext.to_str(), Some("py" | "pyw")))
}
