//! 全项目原生检查之后的有界候选语法观察；永不产生已确认违规或通过。

use crate::discovery::DiscoveryReport;
use crate::go_lint_command::selected_sources_for_candidate;
use crate::grammar_probe_command::read_plain_source;
use crate::grammar_route::route_source;
use crate::syntax_worker_candidate_observation::SyntaxWorkerCandidateObservation;
use crate::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_adapters::bundled_grammar_metadata;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const MAX_FILES: usize = 64;
const MAX_FRAGMENTS: usize = 64;
const MAX_VISIBLE_RECOVERIES: usize = 8;
const MAX_CONCURRENT_WORKERS: usize = 2;

/// 本轮已执行的原生语法相关结果及显式工具；仅用于判断同一源码是否可避免重复解析。
pub struct NativeCoverage<'a> {
    pub node_lint: &'a Value,
    pub python_lint: &'a Value,
    pub go_lint: &'a Value,
    pub erlang_lint: &'a Value,
    pub kotlin_lint: &'a Value,
    pub rust_targets: &'a crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage,
    pub go_tool: Option<&'a Path>,
}

struct CandidateJob {
    relative: String,
    source: Arc<Vec<u8>>,
    language: &'static str,
    scope: &'static str,
    byte_offset: usize,
    end_byte: usize,
    known_limitations: Vec<String>,
}

enum PlannedObservation {
    Immediate(Value),
    Worker(CandidateJob),
}

/// 对已发现源码按方言选择固定资产，受文件、片段和剩余时间限制。
/// 参数为项目根、静态发现、语言选择、全局并发上限、共同截止时间及范围稳定状态；返回不具门禁权威的报告。
pub fn observe(
    root: &Path,
    discovery: &DiscoveryReport,
    native: NativeCoverage<'_>,
    java_only: bool,
    jobs_limit: usize,
    deadline: Instant,
    skip_reason: Option<&str>,
) -> Value {
    let mut paths = BTreeSet::new();
    for (language, evidence) in &discovery.languages {
        if !java_only || language == "java" {
            paths.extend(evidence.source_files.iter());
        }
    }
    if !java_only {
        paths.extend(discovery.ambiguous_source_files.iter());
    }
    observe_selected(
        root,
        &paths.into_iter().cloned().collect(),
        native,
        jobs_limit,
        deadline,
        skip_reason,
    )
}

/// 仅观察调用方已验证的工作区相对文件；不遍历项目，不扩大编辑范围。
/// 参数沿用原生覆盖与共同预算，返回相同候选报告。
pub(crate) fn observe_selected(
    root: &Path,
    selected: &BTreeSet<String>,
    native: NativeCoverage<'_>,
    jobs_limit: usize,
    deadline: Instant,
    skip_reason: Option<&str>,
) -> Value {
    let paths: BTreeSet<&String> = selected.iter().collect();
    let source_file_count = paths.len();
    let observations = Vec::new();
    let mut skipped_count = 0;
    let mut unrouted_count = 0;
    let mut native_preferred_count = 0;
    if let Some(reason) = skip_reason {
        return report(
            "not_run",
            reason,
            source_file_count,
            source_file_count,
            0,
            0,
            observations,
        );
    }
    let manifest = match bundled_grammar_metadata() {
        Ok(manifest) => manifest,
        Err(_) => {
            return report(
                "not_run",
                "grammar_manifest_invalid",
                source_file_count,
                source_file_count,
                0,
                0,
                observations,
            );
        }
    };
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(_) => {
            return report(
                "not_run",
                "worker_executable_unavailable",
                source_file_count,
                source_file_count,
                0,
                0,
                observations,
            );
        }
    };
    let candidate_deadline = deadline.min(Instant::now() + Duration::from_secs(90));
    let cancelled = AtomicBool::new(false);
    let native_go_files =
        if !selected.iter().any(|path| path.ends_with(".go")) || native.go_lint.is_null() {
            BTreeMap::new()
        } else {
            selected_sources_for_candidate(
                root,
                native.go_tool,
                native.go_lint,
                candidate_deadline,
                &cancelled,
            )
            .unwrap_or_default()
        };
    let native_erlang_files = crate::check_erlang_scan::covered_sources(native.erlang_lint);
    let mut planned = Vec::new();
    for (index, relative) in paths.into_iter().enumerate() {
        if index >= MAX_FILES
            || planned.len() >= MAX_FRAGMENTS
            || Instant::now() >= candidate_deadline
            || codeguard_runtime::sigint_cancellation_requested()
        {
            skipped_count += 1;
            continue;
        }
        let source = match read_plain_source(&root.join(relative)) {
            Ok(source) => source,
            Err(reason) => {
                planned.push(PlannedObservation::Immediate(json!({
                    "path":relative,"language":null,"scope":"whole_file","byte_offset":0,
                    "status":"candidate_unavailable","reason":reason,"grammar_qualified":false,
                    "source_sha256":null,"grammar_sha256":null,"recovery_count":0,"recoveries":[],
                    "known_limitations":[]
                })));
                continue;
            }
        };
        if crate::check_kotlin_scan::prefers(native.kotlin_lint, relative)
            || crate::check_eslint_scan::covers(native.node_lint, relative, &source)
            || native_python_covers(native.python_lint, relative, &source)
            || native_go_covers(&native_go_files, relative, &source)
            || native.rust_targets.covers(relative, &source)
            || ((relative.ends_with(".erl") || relative.ends_with(".hrl"))
                && native_erlang_files
                    .get(relative)
                    .is_some_and(|digest| *digest == format!("{:x}", Sha256::digest(&source))))
        {
            native_preferred_count += 1;
            continue;
        }
        let routes = route_source(relative, &source)
            .into_iter()
            .map(|route| {
                (
                    route.language,
                    route.scope,
                    route.byte_offset,
                    route.byte_offset + route.source.len(),
                )
            })
            .collect::<Vec<_>>();
        if routes.is_empty() {
            unrouted_count += 1;
        }
        let source = Arc::new(source);
        for (language, scope, byte_offset, end_byte) in routes {
            if planned.len() >= MAX_FRAGMENTS || Instant::now() >= candidate_deadline {
                skipped_count += 1;
                continue;
            }
            let known_limitations = manifest
                .assets
                .iter()
                .find(|asset| asset.language == language)
                .map(|asset| asset.known_limitations.clone())
                .unwrap_or_default();
            planned.push(PlannedObservation::Worker(CandidateJob {
                relative: relative.clone(),
                source: Arc::clone(&source),
                language,
                scope,
                byte_offset,
                end_byte,
                known_limitations,
            }));
        }
    }
    let mut completed: Vec<Option<Value>> = vec![None; planned.len()];
    let worker_indices = planned
        .iter()
        .enumerate()
        .filter_map(|(index, item)| matches!(item, PlannedObservation::Worker(_)).then_some(index))
        .collect::<Vec<_>>();
    for batch in worker_indices.chunks(jobs_limit.clamp(1, MAX_CONCURRENT_WORKERS)) {
        if Instant::now() >= candidate_deadline
            || codeguard_runtime::sigint_cancellation_requested()
        {
            skipped_count += batch.len();
            continue;
        }
        let results = std::thread::scope(|scope| {
            let handles = batch
                .iter()
                .map(|index| {
                    let PlannedObservation::Worker(job) = &planned[*index] else {
                        unreachable!("worker index must refer to a prepared job")
                    };
                    scope.spawn(|| {
                        run_syntax_worker_candidate(
                            &executable,
                            job.language,
                            &job.relative,
                            &job.source[job.byte_offset..job.end_byte],
                            candidate_deadline,
                            &cancelled,
                        )
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err("syntax_worker_parent_panic".into()))
                })
                .collect::<Vec<_>>()
        });
        for (index, result) in batch.iter().copied().zip(results) {
            let PlannedObservation::Worker(job) = &planned[index] else {
                unreachable!("worker result must match a prepared job")
            };
            completed[index] = Some(candidate_result(root, job, result));
        }
    }
    let observations = planned
        .into_iter()
        .zip(completed)
        .filter_map(|(item, result)| match item {
            PlannedObservation::Immediate(value) => Some(value),
            PlannedObservation::Worker(_) => result,
        })
        .collect::<Vec<_>>();
    let observed = observations
        .iter()
        .any(|item| item["status"] == "candidate_observed");
    let status = if observed {
        "observed_partial"
    } else if observations.is_empty() {
        "not_run"
    } else {
        "attempted_incomplete"
    };
    let reason = if skipped_count > 0 {
        "bounded_scope"
    } else if unrouted_count > 0 {
        "unrouted_source"
    } else if !observed && !observations.is_empty() {
        "candidate_unavailable"
    } else if native_preferred_count > 0 && observations.is_empty() {
        "native_preferred"
    } else {
        "candidate_unqualified"
    };
    report(
        status,
        reason,
        source_file_count,
        skipped_count,
        unrouted_count,
        native_preferred_count,
        observations,
    )
}

fn candidate_result(
    root: &Path,
    job: &CandidateJob,
    result: Result<SyntaxWorkerCandidateObservation, String>,
) -> Value {
    match result {
        Ok(observation) => {
            if read_plain_source(&root.join(&job.relative)).as_deref() != Ok(job.source.as_slice())
            {
                return json!({
                    "path":job.relative,"language":job.language,"scope":job.scope,"byte_offset":job.byte_offset,
                    "status":"candidate_unavailable","reason":"source_changed_during_precheck","grammar_qualified":false,
                    "source_sha256":null,"grammar_sha256":null,"recovery_count":0,"recoveries":[],
                    "known_limitations":job.known_limitations
                });
            }
            // 嵌入片段的 worker 坐标以片段为起点，公开报告还原为文件坐标。
            let base_row = job.source[..job.byte_offset]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count();
            let base_column = job.source[..job.byte_offset]
                .rsplit(|byte| *byte == b'\n')
                .next()
                .map_or(0, <[u8]>::len);
            let recovery_count = observation.recoveries.len();
            let incomplete_reason =
                (observation.precheck.truncated_files > 0).then_some("syntax_recovery_incomplete");
            let recoveries: Vec<Value> = observation.recoveries.iter().take(MAX_VISIBLE_RECOVERIES).map(|recovery| {
                json!({
                    "kind":recovery.kind,
                    "syntax_kind":recovery.syntax_kind,
                    "start_byte":job.byte_offset + recovery.start_byte,
                    "end_byte":job.byte_offset + recovery.end_byte,
                    "start_row":base_row + recovery.start_row,
                    "start_column_byte":if recovery.start_row == 0 { base_column + recovery.start_column_byte } else { recovery.start_column_byte },
                    "end_row":base_row + recovery.end_row,
                    "end_column_byte":if recovery.end_row == 0 { base_column + recovery.end_column_byte } else { recovery.end_column_byte },
                })
            }).collect();
            json!({
                "path":job.relative,"language":job.language,"scope":job.scope,"byte_offset":job.byte_offset,
                "status":"candidate_observed","reason":incomplete_reason,"grammar_qualified":false,
                "source_sha256":observation.source_sha256,"grammar_sha256":observation.grammar_sha256,
                "recovery_count":recovery_count,"recoveries":recoveries,
                "known_limitations":job.known_limitations
            })
        }
        Err(reason) => json!({
            "path":job.relative,"language":job.language,"scope":job.scope,"byte_offset":job.byte_offset,
            "status":"candidate_unavailable","reason":reason,"grammar_qualified":false,
            "source_sha256":null,"grammar_sha256":null,"recovery_count":0,"recoveries":[],
            "known_limitations":job.known_limitations
        }),
    }
}

fn native_go_covers(files: &BTreeMap<String, String>, relative: &str, source: &[u8]) -> bool {
    relative.ends_with(".go")
        && files
            .get(relative)
            .is_some_and(|digest| *digest == format!("{:x}", Sha256::digest(source)))
}

// 仅本轮 Ruff 已完整扫描同一份 Python 字节时跳过重复解析；配置发现不算执行。
fn native_python_covers(python_lint: &Value, relative: &str, source: &[u8]) -> bool {
    if !relative.ends_with(".py") {
        return false;
    }
    let digest = format!("{:x}", Sha256::digest(source));
    python_lint["files"].as_array().is_some_and(|files| {
        files.iter().any(|file| {
            file["path"] == relative
                && matches!(
                    file["run_status"].as_str(),
                    Some("passed" | "findings" | "suppressed")
                )
                && file["source_sha256"] == digest
                && file["tool_sha256"].as_str().is_some()
                && file["config_sha256"].as_str().is_some()
        })
    })
}

fn report(
    status: &str,
    reason: &str,
    source_file_count: usize,
    skipped_count: usize,
    unrouted_count: usize,
    native_preferred_count: usize,
    observations: Vec<Value>,
) -> Value {
    json!({
        "status":status,"reason":reason,"execution_phase":"after_native",
        "authority":"candidate_unqualified","delivery_decision":"incomplete",
        "source_file_count":source_file_count,"skipped_count":skipped_count,
        "unrouted_count":unrouted_count,"native_preferred_count":native_preferred_count,"observations":observations,
        "next_action":"核对适用语言版本与原生 lint/编译器；候选恢复节点不是已确认违规，零恢复也不表示完整通过"
    })
}

#[cfg(test)]
mod tests {
    use super::native_python_covers;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    #[test]
    fn only_completed_matching_native_python_scan_preempts_candidate() {
        let source = b"import os\n";
        let digest = format!("{:x}", Sha256::digest(source));
        for status in ["passed", "findings", "suppressed"] {
            let native = json!({"files":[{"path":"app.py","run_status":status,
                "source_sha256":digest,"tool_sha256":"tool","config_sha256":"config"}]});
            assert!(native_python_covers(&native, "app.py", source));
            assert!(!native_python_covers(&native, "other.py", source));
            assert!(!native_python_covers(&native, "app.rs", source));
            assert!(!native_python_covers(&native, "app.py", b"changed\n"));
        }
        for status in ["incomplete", "not_run"] {
            let native = json!({"files":[{"path":"app.py","run_status":status,
                "source_sha256":digest,"tool_sha256":"tool","config_sha256":"config"}]});
            assert!(!native_python_covers(&native, "app.py", source));
        }
    }
}
