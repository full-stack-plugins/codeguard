//! 开发期原生/WASM 差分回放；使用已支持的原生观察器，不授予独立 holdout 或语言资格。
use crate::grammar_evaluation::{classify_probe, validate_corpus};
use crate::grammar_native_checker::GrammarNativeChecker;
use crate::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 映射受控原生观察的完成状态；参数必须来自适配器，不接受项目自报批准。
/// 完整零诊断为 Some(true)，已定位原生语法诊断为 Some(false)；仅上下文或运行失败为 None。
/// Kotlin 混合上下文阻塞仍保留正向语法证据，但不提升执行完整性。
pub fn classify_native(native: &Value) -> Option<bool> {
    match native["status"].as_str() {
        Some("completed") => Some(true),
        Some("diagnostics_observed") => Some(false),
        Some("incomplete")
            if crate::task_resolution_evidence_shape::kotlin_syntax_present(native) =>
        {
            Some(false)
        }
        _ => None,
    }
}

/// 在完整固定语料中回放显式选中的原生工具及对应 WASM；未选语言保留在库存。
/// 参数为固定程序、语料字节、语言到绝对工具路径、总截止时间和取消标记。
/// 返回脱敏差分报告；原生进程和WASM worker共用请求取消令牌，文件摘要I/O仍无硬中断保障。
pub fn replay_native_corpus(
    executable: &Path,
    corpus_bytes: &[u8],
    tools: &BTreeMap<String, PathBuf>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    let corpus = validate_corpus(corpus_bytes)?;
    if tools.is_empty() || !executable.is_absolute() {
        return Err("native_grammar_selection_invalid".into());
    }
    let program_sha = artifact_hash(executable, 256 * 1024 * 1024)?;
    let cwd = std::env::current_dir().map_err(|_| "native_grammar_cwd_unavailable")?;
    // 全部显式工具在第一次调用前冻结；不从 PATH 自动安装或补充另一工具。
    let mut frozen = BTreeMap::new();
    for (language, tool) in tools {
        let checker = checker(language).ok_or("native_grammar_language_unsupported")?;
        if !tool.is_absolute() {
            return Err("native_grammar_tool_not_absolute".into());
        }
        let path = tool
            .canonicalize()
            .map_err(|_| "native_grammar_tool_unavailable")?;
        let metadata = std::fs::metadata(&path).map_err(|_| "native_grammar_tool_unavailable")?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err("native_grammar_tool_unavailable".into());
        }
        let sha = artifact_hash(&path, checker.artifact_budget())?;
        frozen.insert(language.clone(), (checker, path, sha));
    }
    let manifest: Value = serde_json::from_slice(include_bytes!("../../../grammars/manifest.json"))
        .map_err(|_| "native_grammar_manifest_invalid")?;
    let javascript_selected = tools.contains_key("javascript");
    let ruby_selected = tools.contains_key("ruby");
    let measure_structure = tools.contains_key("python") || javascript_selected || ruby_selected;
    let mut cases = Vec::new();
    for case in &corpus.cases {
        let Some((checker, tool, tool_sha)) = frozen.get(&case.language) else {
            continue;
        };
        let admitted = !cancelled.load(Ordering::Relaxed) && Instant::now() < deadline;
        let native_started = Instant::now();
        let tool_ready = artifact_hash(tool, checker.artifact_budget())
            .is_ok_and(|sha| sha == *tool_sha)
            && tools
                .get(&case.language)
                .and_then(|input| input.canonicalize().ok())
                .as_ref()
                == Some(tool);
        let native_attempted = admitted
            && tool_ready
            && !cancelled.load(Ordering::Relaxed)
            && Instant::now() < deadline;
        let native = if native_attempted {
            // 保留原请求入口供观察器复核别名；只传规范路径会隐藏版本调用中的重定向。
            checker.observe(
                &tools[&case.language],
                case.source.as_bytes(),
                &cwd,
                deadline,
                cancelled,
            )
        } else {
            json!({"status":"not_run","reason":if !tool_ready {"native_grammar_tool_changed"}else if cancelled.load(Ordering::Relaxed) {"request_cancelled"} else {"request_deadline_exceeded"},"version":null,"tool_sha256":null,"diagnostics":[]})
        };
        let native_us = elapsed_us(native_started);
        let tool_current = artifact_hash(tool, checker.artifact_budget())
            .is_ok_and(|s| s == *tool_sha)
            && native["tool_sha256"] == *tool_sha
            && native["version"] == checker.version();
        let native_class = tool_current.then(|| classify_native(&native)).flatten();
        let wasm_started = Instant::now();
        let observation =
            if !admitted || cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
                Err(if cancelled.load(Ordering::Relaxed) {
                    "request_cancelled"
                } else {
                    "request_deadline_exceeded"
                }
                .to_owned())
            } else {
                run_syntax_worker_candidate(
                    executable,
                    &case.language,
                    &case.id,
                    case.source.as_bytes(),
                    deadline,
                    cancelled,
                )
            };
        let (wasm_class, recovery_count, wasm_reason, structures) = match observation {
            Ok(observation) => {
                let truncated = observation.precheck.truncated_files > 0;
                (
                    classify_probe(observation.recoveries.len(), truncated),
                    Some(observation.recoveries.len()),
                    truncated.then_some("syntax_recovery_incomplete".to_owned()),
                    Some(observation.structural_observations),
                )
            }
            Err(reason) => (None, None, Some(reason), None),
        };
        let asset = manifest["assets"]
            .as_array()
            .and_then(|assets| assets.iter().find(|a| a["language"] == case.language))
            .ok_or("native_grammar_asset_missing")?;
        let mut row = json!({"id":case.id,"language":case.language,"cohort":case.cohort,"origin":case.origin,
            "source_sha256":case.source_sha256,"grammar_sha256":asset["sha256"],"fixture_expected_valid":case.expected_valid,"fixture_label":case.label,
            "native_attempted":native_attempted,"native":native,"native_classification":name(native_class),"native_identity_current":tool_current,
            "native_elapsed_us":native_us,"wasm_classification":name(wasm_class),"wasm_recovery_count":recovery_count,"wasm_reason":wasm_reason,
            "wasm_elapsed_us":elapsed_us(wasm_started),"comparison":comparison(native_class,wasm_class),
            "fixture_native_disagreement":native_class.map(|valid|valid!=case.expected_valid)});
        if measure_structure {
            // 原始恢复统计保持不变，结构规则仅补充独立的候选层测量。
            let combined = combined_candidate(wasm_class, structures.as_ref().map(Vec::len));
            row["structural_observations"] = json!(structures);
            row["combined_candidate_classification"] = json!(name(combined));
            row["combined_candidate_comparison"] = json!(comparison(native_class, combined));
        }
        cases.push(row);
    }
    let program_stable =
        artifact_hash(executable, 256 * 1024 * 1024).is_ok_and(|s| s == program_sha);
    let mut inventory = Vec::new();
    for asset in manifest["assets"]
        .as_array()
        .ok_or("native_grammar_manifest_invalid")?
    {
        let language = asset["language"]
            .as_str()
            .ok_or("native_grammar_manifest_invalid")?;
        if let Some((checker, tool, sha)) = frozen.get(language) {
            let stable = artifact_hash(tool, checker.artifact_budget())
                .is_ok_and(|current| current == *sha)
                && tools
                    .get(language)
                    .and_then(|input| input.canonicalize().ok())
                    .as_ref()
                    == Some(tool);
            for row in cases.iter_mut().filter(|c| c["language"] == language) {
                // 批次结束时撤回已更换制品或入口的原生分类；失败样本保留在分母。
                if !stable {
                    row["native_classification"] = json!("unknown");
                    row["native_identity_current"] = json!(false);
                    row["fixture_native_disagreement"] = Value::Null;
                }
                if !program_stable {
                    row["wasm_classification"] = json!("unknown");
                    row["wasm_recovery_count"] = Value::Null;
                    row["wasm_reason"] = json!("grammar_evaluation_program_changed");
                    if measure_structure {
                        row["structural_observations"] = Value::Null;
                        row["combined_candidate_classification"] = json!("unknown");
                    }
                }
                row["comparison"] = json!(comparison(
                    parse_name(&row["native_classification"]),
                    parse_name(&row["wasm_classification"])
                ));
                if measure_structure {
                    // 原生身份撤回同时影响两层比较，不能沿用先前的有效分母。
                    row["combined_candidate_comparison"] = json!(comparison(
                        parse_name(&row["native_classification"]),
                        parse_name(&row["combined_candidate_classification"]),
                    ));
                }
            }
            let local: Vec<&Value> = cases.iter().filter(|c| c["language"] == language).collect();
            let count = |key: &str, value: &str| local.iter().filter(|c| c[key] == value).count();
            let mut language_row = json!({"language":language,"grammar_sha256":asset["sha256"],"native_selected":true,"native_version":checker.version(),"tool_sha256":sha,"tool_stable":stable,
                "sample_count":local.len(),"native_unknown_count":count("native_classification","unknown"),"wasm_unknown_count":count("wasm_classification","unknown"),
                "compared_count":local.iter().filter(|c|c["comparison"]!="unknown").count(),"tp":count("comparison","true_positive"),"fp":count("comparison","false_positive"),"fn":count("comparison","false_negative"),"tn":count("comparison","true_negative"),
                "fixture_native_disagreement_count":local.iter().filter(|c| c["fixture_native_disagreement"]==true).count(),"grammar_qualified":false});
            if measure_structure {
                language_row["combined_candidate"] = json!({
                    "compared_count":local.iter().filter(|c|c["combined_candidate_comparison"]!="unknown").count(),
                    "unknown_count":count("combined_candidate_comparison","unknown"),
                    "tp":count("combined_candidate_comparison","true_positive"),
                    "fp":count("combined_candidate_comparison","false_positive"),
                    "fn":count("combined_candidate_comparison","false_negative"),
                    "tn":count("combined_candidate_comparison","true_negative")
                });
            }
            inventory.push(language_row);
        } else {
            inventory.push(json!({"language":language,"grammar_sha256":asset["sha256"],"native_selected":false,"reason":if checker(language).is_some(){"explicit_native_tool_not_selected"}else{"native_differential_adapter_unavailable"},"grammar_qualified":false}));
        }
    }
    let mut report = json!({"schema_version":if ruby_selected {"0.5.0"}else if javascript_selected {"0.4.0"}else if measure_structure {"0.3.0"}else{"0.1.0"},"report_type":"native_grammar_differential","status":"incomplete","delivery_decision":"not_evaluated",
        "authority":"development_native_differential_only","native_adapter_reused":!(javascript_selected || ruby_selected),"independent_holdout":false,"grammar_qualified_count":0,
        "corpus_sha256":digest(corpus_bytes),"manifest_sha256":corpus.manifest_sha256,"program_sha256":program_sha,"program_stable":program_stable,
        "language_count":inventory.len(),"selected_language_count":frozen.len(),"sample_count":cases.len(),"languages":inventory,"cases":cases});
    if measure_structure {
        report["combined_candidate_authority"] = json!("native_confirmation_required");
    }
    Ok(report)
}

// 隐藏恢复、截断或失败不能被正向结构观察升级为可判定结果。
fn combined_candidate(parser: Option<bool>, structure_count: Option<usize>) -> Option<bool> {
    let parser_valid = parser?;
    let structure_count = structure_count?;
    Some(parser_valid && structure_count == 0)
}
fn checker(language: &str) -> Option<GrammarNativeChecker> {
    GrammarNativeChecker::for_language(language)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn artifact_hash(path: &Path, budget: u64) -> Result<String, String> {
    read_bounded_regular_file(path, budget)
        .map(|bytes| digest(&bytes))
        .map_err(|_| "native_grammar_artifact_unavailable".into())
}
fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u64::MAX as u128) as u64
}
fn name(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "valid",
        Some(false) => "invalid",
        None => "unknown",
    }
}
fn parse_name(value: &Value) -> Option<bool> {
    match value.as_str() {
        Some("valid") => Some(true),
        Some("invalid") => Some(false),
        _ => None,
    }
}
fn comparison(native: Option<bool>, wasm: Option<bool>) -> &'static str {
    match (native, wasm) {
        (Some(false), Some(false)) => "true_positive",
        (Some(true), Some(false)) => "false_positive",
        (Some(false), Some(true)) => "false_negative",
        (Some(true), Some(true)) => "true_negative",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn combined_candidate_requires_complete_parser_and_structure_observation() {
        for (parser, count, expected) in [
            (Some(true), Some(0), Some(true)),
            (Some(true), Some(1), Some(false)),
            (Some(false), Some(0), Some(false)),
            (Some(false), Some(1), Some(false)),
            (None, Some(0), None),
            (None, Some(1), None),
            (Some(true), None, None),
            (Some(false), None, None),
        ] {
            assert_eq!(super::combined_candidate(parser, count), expected);
        }
    }
}
