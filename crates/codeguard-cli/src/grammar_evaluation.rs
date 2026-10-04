//! 开发期全 grammar 回放；复用隔离 worker 与 Core 分层计算，不签发资格。

pub use crate::grammar_evaluation_corpus::GrammarEvaluationCorpus;
use codeguard_adapters::parse_unique_json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MANIFEST: &[u8] = include_bytes!("../../../grammars/manifest.json");

/// 校验固定语料的结构、范围与字节绑定；在任何进程启动前调用。
/// 参数为至多 16 MiB 的原始 JSON；返回包含全部随仓语言的语料或具体原因。
pub fn validate_corpus(bytes: &[u8]) -> Result<GrammarEvaluationCorpus, String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("grammar_evaluation_corpus_too_large".into());
    }
    let value = parse_unique_json(bytes).map_err(str::to_owned)?;
    let modern = value["schema_version"] == "0.2.0";
    if let Some(cases) = value["cases"].as_array() {
        if cases
            .iter()
            .any(|case| case.get("cohort").is_some() != modern)
        {
            return Err("grammar_evaluation_cohort_version_mismatch".into());
        }
    }
    let corpus: GrammarEvaluationCorpus =
        serde_json::from_value(value).map_err(|_| "grammar_evaluation_corpus_shape_invalid")?;
    if !matches!(corpus.schema_version.as_str(), "0.1.0" | "0.2.0")
        || corpus.corpus_type != "grammar_regression"
        || corpus.manifest_sha256 != digest(MANIFEST)
        || corpus.cases.is_empty()
        || corpus.cases.len() > 4096
    {
        return Err("grammar_evaluation_corpus_identity_invalid".into());
    }
    let manifest: Value = serde_json::from_slice(MANIFEST).map_err(|e| e.to_string())?;
    let expected: BTreeSet<&str> = manifest["assets"]
        .as_array()
        .ok_or("grammar_evaluation_manifest_invalid")?
        .iter()
        .filter_map(|a| a["language"].as_str())
        .collect();
    let mut ids = BTreeSet::new();
    let mut languages = BTreeSet::new();
    for case in &corpus.cases {
        if !valid_id(&case.id)
            || !ids.insert(case.id.as_str())
            || !expected.contains(case.language.as_str())
            || case.source.len() > 1024 * 1024
            || case.source_sha256 != digest(case.source.as_bytes())
            || !matches!(case.label.as_str(), "regression" | "pending")
            || !matches!(
                case.cohort.as_str(),
                "repository_regression" | "upstream_grammar_regression" | "provisional_syntax"
            )
            || (case.cohort == "provisional_syntax" && case.label != "pending")
            || case.origin.is_empty()
            || case.origin.len() > 512
            || case.origin.chars().any(char::is_control)
        {
            return Err(format!("grammar_evaluation_case_invalid:{}", case.id));
        }
        languages.insert(case.language.as_str());
    }
    if languages != expected {
        return Err("grammar_evaluation_language_scope_mismatch".into());
    }
    Ok(corpus)
}

/// 将恢复扫描映射为语法分类；截断或隐藏错误始终返回未知。
/// 参数为恢复节点数量和扫描是否截断；Some(true) 仅表示此样本无恢复节点。
pub fn classify_probe(recovery_count: usize, truncated: bool) -> Option<bool> {
    (!truncated).then_some(recovery_count == 0)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

#[cfg(all(feature = "wasm-precheck", unix))]
pub use replay::replay_corpus;

#[cfg(all(feature = "wasm-precheck", unix))]
mod replay {
    use super::{MANIFEST, classify_probe, digest, validate_corpus};
    use crate::syntax_worker_runner::run_syntax_worker_candidate;
    use codeguard_core::{
        EvaluationCase, EvaluationOutcome, EvaluationThresholds, OracleDecision, evaluate_quality,
    };
    use codeguard_runtime::read_bounded_regular_file;
    use serde_json::{Value, json};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    /// 用固定程序与共同 deadline 顺序回放全部随仓 grammar，并生成逐语言报告。
    /// 输入为显式绝对程序路径、固定语料字节、deadline 和取消标记；返回脱敏报告。
    /// 此入口仅为开发验收，不运行原生 oracle，也不改变 grammar 资格或项目门禁。
    pub fn replay_corpus(
        executable: &Path,
        corpus_bytes: &[u8],
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Value, String> {
        // 全部输入先校验，错误语料不能先运行一部分再报参数错。
        let corpus = validate_corpus(corpus_bytes)?;
        if !executable.is_absolute() {
            return Err("grammar_evaluation_executable_not_absolute".into());
        }
        let program_sha = hash_program(executable)?;
        let mut rows = Vec::new();
        let mut evaluations = Vec::new();
        for case in &corpus.cases {
            let started = Instant::now();
            let observation = if cancelled.load(Ordering::Relaxed) {
                Err("request_cancelled".into())
            } else if Instant::now() >= deadline {
                Err("request_deadline_exceeded".into())
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
            let (classification, recovery_count, reason, attempted) = match observation {
                Ok(obs) => {
                    let truncated = obs.precheck.truncated_files > 0;
                    (
                        classify_probe(obs.recoveries.len(), truncated),
                        Some(obs.recoveries.len()),
                        truncated.then_some("syntax_recovery_incomplete".to_owned()),
                        true,
                    )
                }
                Err(reason) => {
                    let attempted = !matches!(
                        reason.as_str(),
                        "request_cancelled" | "request_deadline_exceeded"
                    );
                    (None, None, Some(reason), attempted)
                }
            };
            // 每个样本至多一个“存在语法异常”事件；不是精确规则实例召回率。
            let finding = format!("{}:syntax", case.id);
            evaluations.push(EvaluationCase {
                id: case.id.clone(),
                cohort: case.cohort.clone(),
                language: case.language.clone(),
                category: "lint".into(),
                adapter_id: "wasm.syntax.candidate".into(),
                oracle: if case.label == "regression" {
                    OracleDecision::Regression
                } else {
                    OracleDecision::Pending
                },
                expected_findings: if case.expected_valid {
                    vec![]
                } else {
                    vec![finding.clone()]
                },
                observed_findings: if classification == Some(false) {
                    vec![finding]
                } else {
                    vec![]
                },
                expected_targets: vec![case.id.clone()],
                observed_targets: vec![case.id.clone()],
                expected_complete: true,
                observed_complete: classification.is_some(),
            });
            rows.push(json!({"id":case.id,"language":case.language,"source_sha256":case.source_sha256,
                "label":case.label,"origin":case.origin,"expected_valid":case.expected_valid,
                "classification":classification_name(classification),"recovery_count":recovery_count,
                "reason":reason,"attempted":attempted,"elapsed_us":started.elapsed().as_micros().min(u64::MAX as u128) as u64}));
            if corpus.schema_version == "0.2.0" {
                rows.last_mut().ok_or("grammar_evaluation_row_missing")?["cohort"] =
                    json!(case.cohort);
            }
        }
        let program_stable = hash_program(executable).is_ok_and(|current| current == program_sha);
        if !program_stable {
            for (row, evaluation) in rows.iter_mut().zip(&mut evaluations) {
                row["classification"] = json!("unknown");
                row["recovery_count"] = Value::Null;
                row["reason"] = json!("grammar_evaluation_program_changed");
                evaluation.observed_complete = false;
                evaluation.observed_findings.clear();
            }
        }
        // 固定规格下界；未批准的开发语料不因此变成发布验收。
        let quality = evaluate_quality(
            &evaluations,
            EvaluationThresholds {
                minimum_observed_findings: 200,
                precision_wilson_lower_bound: 0.98,
                max_false_negatives: 0,
            },
        )?;
        let manifest: Value = serde_json::from_slice(MANIFEST).map_err(|e| e.to_string())?;
        let mut languages = Vec::new();
        for stratum in &quality.strata {
            let local: Vec<&Value> = rows
                .iter()
                .filter(|row| row["language"] == stratum.language)
                .filter(|row| corpus.schema_version == "0.1.0" || row["cohort"] == stratum.cohort)
                .collect();
            let asset = manifest["assets"]
                .as_array()
                .ok_or("grammar_evaluation_manifest_invalid")?
                .iter()
                .find(|asset| asset["language"] == stratum.language)
                .ok_or("grammar_evaluation_asset_missing")?;
            let n = local.len();
            let unknown = local
                .iter()
                .filter(|r| r["classification"] == "unknown")
                .count();
            let mut times: Vec<u64> = local
                .iter()
                .filter(|r| r["attempted"] == true)
                .filter_map(|r| r["elapsed_us"].as_u64())
                .collect();
            times.sort_unstable();
            let counts = &stratum.counts;
            let tn = local
                .iter()
                .filter(|r| {
                    r["label"] == "regression"
                        && r["expected_valid"] == true
                        && r["classification"] == "valid"
                })
                .count();
            languages.push(json!({"language":stratum.language,"grammar_sha256":asset["sha256"],
                "sample_count":n,"decidable_count":n-unknown,"unknown_count":unknown,
                "pending_label_count":counts.pending_cases,"evaluated_count":counts.evaluated_cases,
                "tp":counts.tp,"fp":counts.fp,"fn":counts.false_negatives,"tn":tn,
                "fixture_false_clean_count":counts.false_pass_cases,
                "precision":stratum.precision.map(|p| json!({"point":p.point,"lower_95":p.lower_95,"upper_95":p.upper_95})),
                "recall":stratum.recall,"completion_rate":(n-unknown) as f64/n as f64,
                "fixture_outcome":outcome_name(stratum.outcome),"grammar_qualified":false,
                "performance":{"scope":"sequential_cold_worker_wall_time","measured_count":times.len(),
                    "p50_us":percentile(&times,50),"p95_us":percentile(&times,95)},
                "next_action":"resolve_disagreements_and_unknowns_then_collect_independent_native_holdout"}));
            if corpus.schema_version == "0.2.0" {
                let row = languages
                    .last_mut()
                    .ok_or("grammar_evaluation_language_missing")?;
                row["cohort"] = json!(stratum.cohort);
                let valid = local.iter().filter(|c| c["expected_valid"] == true).count();
                row["selected_valid_count"] = json!(valid);
                row["selected_invalid_count"] = json!(n - valid);
            }
        }
        let cohort_count = languages.len();
        if corpus.schema_version == "0.2.0" {
            languages = separate_cohorts(&languages, &rows)?;
        }
        let mut report = json!({"schema_version":corpus.schema_version,"report_type":"grammar_regression_evaluation",
            "authority":"repository_regression_only","native_oracle_executed":false,"independent_holdout":false,
            "metric_scope":"sample_level_syntax_classification","status":"incomplete","delivery_decision":"not_evaluated",
            "grammar_qualified_count":0,"manifest_sha256":corpus.manifest_sha256,"corpus_sha256":digest(corpus_bytes),
            "program_sha256":program_sha,"program_stable":program_stable,"sample_count":rows.len(),
            "language_count":languages.len(),"fixture_outcome":outcome_name(quality.overall),
            "thresholds":{"minimum_observed_findings":200,"precision_wilson_lower_bound":0.98,"max_false_negatives":0,"approval":"not_verified"},
            "languages":languages,"cases":rows});
        if corpus.schema_version == "0.2.0" {
            report["cohort_count"] = json!(cohort_count);
            report["cohort_policy"] = json!("separate_sources_no_pooled_precision");
        }
        Ok(report)
    }

    fn separate_cohorts(cohorts: &[Value], rows: &[Value]) -> Result<Vec<Value>, String> {
        let languages: std::collections::BTreeSet<&str> = cohorts
            .iter()
            .filter_map(|c| c["language"].as_str())
            .collect();
        let mut result = Vec::new();
        for language in languages {
            let groups: Vec<Value> = cohorts
                .iter()
                .filter(|c| c["language"] == language)
                .cloned()
                .collect();
            let mut summary = groups
                .first()
                .cloned()
                .ok_or("grammar_evaluation_cohort_missing")?;
            summary
                .as_object_mut()
                .ok_or("grammar_evaluation_summary_invalid")?
                .remove("cohort");
            for key in [
                "sample_count",
                "selected_valid_count",
                "selected_invalid_count",
                "decidable_count",
                "unknown_count",
                "pending_label_count",
                "evaluated_count",
                "tp",
                "fp",
                "fn",
                "tn",
                "fixture_false_clean_count",
            ] {
                summary[key] = json!(groups.iter().filter_map(|c| c[key].as_u64()).sum::<u64>());
            }
            let n = summary["sample_count"]
                .as_u64()
                .ok_or("grammar_evaluation_sample_count_invalid")?;
            summary["completion_rate"] = json!(
                summary["decidable_count"]
                    .as_u64()
                    .ok_or("grammar_evaluation_count_invalid")? as f64
                    / n as f64
            );
            summary["fixture_outcome"] = json!(if groups
                .iter()
                .any(|c| c["fixture_outcome"] == "fails_fixture_threshold")
            {
                "fails_fixture_threshold"
            } else {
                "insufficient_evidence"
            });
            summary["metric_aggregation"] = json!(if groups.len() == 1 {
                "single_cohort"
            } else {
                "not_pooled"
            });
            if groups.len() > 1 {
                summary["precision"] = Value::Null;
                summary["recall"] = Value::Null;
            }
            let mut times: Vec<u64> = rows
                .iter()
                .filter(|r| r["language"] == language && r["attempted"] == true)
                .filter_map(|r| r["elapsed_us"].as_u64())
                .collect();
            times.sort_unstable();
            summary["performance"] = json!({"scope":"sequential_cold_worker_wall_time","measured_count":times.len(),
                "p50_us":percentile(&times,50),"p95_us":percentile(&times,95)});
            summary["cohorts"] = json!(groups);
            result.push(summary);
        }
        Ok(result)
    }

    fn hash_program(path: &Path) -> Result<String, String> {
        read_bounded_regular_file(path, 256 * 1024 * 1024)
            .map(|b| digest(&b))
            .map_err(|_| "grammar_evaluation_executable_unavailable".into())
    }

    fn classification_name(classification: Option<bool>) -> &'static str {
        match classification {
            Some(true) => "valid",
            Some(false) => "invalid",
            None => "unknown",
        }
    }

    fn outcome_name(outcome: EvaluationOutcome) -> &'static str {
        match outcome {
            EvaluationOutcome::MeetsThreshold => "meets_fixture_threshold",
            EvaluationOutcome::FailsThreshold => "fails_fixture_threshold",
            EvaluationOutcome::InsufficientEvidence => "insufficient_evidence",
        }
    }

    fn percentile(values: &[u64], percentage: usize) -> Option<u64> {
        (!values.is_empty()).then(|| values[(values.len() * percentage).div_ceil(100) - 1])
    }
}
