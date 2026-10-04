//! 所有固定 grammar 的有界疑似观察复用既有工作台；不签发源码违规或关闭任务。
use codeguard_adapters::bundled_grammar_candidates;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Component, Path},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// 同步工作区 root 下本轮 syntax 候选，沿用 deadline；返回真实任务引用和失败范围。
/// 完整零恢复不创建新阻塞；无法定位的未完成观察需要恢复检查能力，持久化故障不能伪造任务。
pub(crate) fn persist(root: &Path, syntax: &Value, deadline: Instant) -> Value {
    let mut groups = BTreeMap::<(String, String), Vec<Value>>::new();
    for row in syntax["observations"].as_array().into_iter().flatten() {
        if row["status"] != "candidate_observed"
            || (row["recovery_count"].as_u64().unwrap_or(0) == 0
                && row["reason"] != "syntax_recovery_incomplete")
        {
            continue;
        }
        if let (Some(path), Some(language)) = (row["path"].as_str(), row["language"].as_str()) {
            groups
                .entry((path.into(), language.into()))
                .or_default()
                .push(row.clone());
        }
    }
    let mut tasks = Vec::new();
    let mut failures = Vec::new();
    let mut new_blockers = 0;
    for ((path, language), rows) in groups {
        let result = (|| {
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                return Err("deadline_or_cancelled");
            }
            let baseline = crate::workspace_refresh::read_workspace_baseline(root)
                .map_err(|_| "workspace_invalid")?;
            let workspace = baseline
                .as_ref()
                .and_then(|b| b.workspace_id())
                .ok_or("workspace_not_initialized")?;
            let (checker, reason, fingerprint) = identity(workspace, &path, &language);
            let id = format!("CG-B-{}", &fingerprint[..32]);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "clock_unavailable")?
                .as_nanos();
            // 新版只扩展无位置的未完成观察；旧有可定位观察仍使用原协议和稳定身份。
            let schema_version = if rows.iter().any(|row| row["recovery_count"] == 0) {
                "0.3.0"
            } else {
                "0.1.0"
            };
            let report = json!({"schema_version":schema_version, "report_type":"syntax_confirmation_observation",
                "workspace_binding":"bound", "workspace_id":workspace, "run_id":format!("syntax-confirm-{}-{nanos}", std::process::id()),
                "authority":"local_unverified", "coverage_proven":false, "delivery_decision":"not_evaluated", "execution":"incomplete",
                "checker_id":checker, "reason_code":reason, "blocker_id":id, "fingerprint":fingerprint,
                "build_root":".", "scope":path, "language":language, "affected_paths":[path], "observations":rows});
            if !valid_report(root, workspace, &report) {
                return Err("syntax_confirmation_report_invalid");
            }
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                return Err("deadline_or_cancelled");
            }
            crate::work_sync::save_local_report(root, &report)?;
            let summary = crate::work_sync::sync_local_workspace(root)?;
            if summary.failed_reports != 0 {
                return Err("syntax_confirmation_sync_incomplete");
            }
            if !root
                .join(".codeguard/tasks")
                .join(format!("{id}.md"))
                .is_file()
            {
                return Err("syntax_confirmation_task_missing");
            }
            new_blockers += summary.new_blockers;
            Ok(json!({"task_id":id,"path":path,"language":language}))
        })();
        match result {
            Ok(task) => tasks.push(task),
            Err(reason) => failures.push(json!({"path":path,"language":language,"reason":reason})),
        }
    }
    json!({"status":if failures.is_empty() {"synced_partial"} else {"incomplete"},"tasks":tasks,"failures":failures,"new_blockers":new_blockers})
}

/// 校验候选任务的工作区、固定 grammar、原字节位置和当前源码；不验证质量批准。
pub(crate) fn valid_report(root: &Path, workspace: &str, report: &Value) -> bool {
    if matches!(
        report["schema_version"].as_str(),
        Some("0.2.0" | "0.4.0" | "0.5.0")
    ) {
        return crate::native_syntax_confirmation::valid_history_report(root, workspace, report)
            && crate::syntax_task_recheck::inputs_current(root, &report["native_evidence"]);
    }
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "execution",
        "checker_id",
        "reason_code",
        "blocker_id",
        "fingerprint",
        "build_root",
        "scope",
        "language",
        "affected_paths",
        "observations",
    ];
    if !exact_keys(report, &keys) {
        return false;
    }
    let Some(path) = report["scope"].as_str().filter(|p| safe_path(p)) else {
        return false;
    };
    let Some(language) = report["language"].as_str() else {
        return false;
    };
    let (checker, reason, fingerprint) = identity(workspace, path, language);
    if !matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.3.0"))
        || report["report_type"] != "syntax_confirmation_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["execution"] != "incomplete"
        || report["checker_id"] != checker
        || report["reason_code"] != reason
        || report["fingerprint"] != fingerprint
        || report["blocker_id"] != format!("CG-B-{}", &fingerprint[..32])
        || report["build_root"] != "."
        || report["affected_paths"] != json!([path])
    {
        return false;
    }
    let Some(run) = report["run_id"]
        .as_str()
        .and_then(|r| r.strip_prefix("syntax-confirm-"))
    else {
        return false;
    };
    if !run
        .split_once('-')
        .is_some_and(|(pid, nanos)| pid.parse::<u32>().is_ok() && nanos.parse::<u128>().is_ok())
    {
        return false;
    }
    let source = root.join(path);
    if source.canonicalize().ok().as_deref() != Some(source.as_path()) {
        return false;
    }
    let Ok(bytes) = read_bounded_regular_file(&source, 1024 * 1024) else {
        return false;
    };
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let Some(grammar) = bundled_grammar_candidates()
        .ok()
        .and_then(|m| m.assets.into_iter().find(|a| a.language == language))
    else {
        return false;
    };
    let routes = crate::grammar_route::route_source(path, &bytes);
    let Some(rows) = report["observations"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 64)
    else {
        return false;
    };
    let mut offsets = std::collections::BTreeSet::new();
    rows.iter().all(|row| {
        if !exact_keys(
            row,
            &[
                "path",
                "language",
                "scope",
                "byte_offset",
                "status",
                "reason",
                "grammar_qualified",
                "source_sha256",
                "grammar_sha256",
                "recovery_count",
                "recoveries",
                "known_limitations",
            ],
        ) || row["path"] != path
            || row["language"] != language
            || row["status"] != "candidate_observed"
            || row["grammar_qualified"] != false
            || row["source_sha256"] != sha
            || row["grammar_sha256"] != grammar.sha256
            || row["known_limitations"] != json!(grammar.known_limitations)
            || !(row["reason"].is_null() || row["reason"] == "syntax_recovery_incomplete")
        {
            return false;
        }
        let Some(route) = routes.iter().find(|r| {
            r.language == language && row["scope"] == r.scope && row["byte_offset"] == r.byte_offset
        }) else {
            return false;
        };
        if !offsets.insert(route.byte_offset) {
            return false;
        }
        let Some(count) = row["recovery_count"].as_u64().filter(|c| {
            *c <= 4096
                && (*c > 0
                    || (report["schema_version"] == "0.3.0"
                        && row["reason"] == "syntax_recovery_incomplete"))
        }) else {
            return false;
        };
        let Some(recoveries) = row["recoveries"]
            .as_array()
            .filter(|r| r.len() == (count as usize).min(8))
        else {
            return false;
        };
        recoveries.iter().all(|r| {
            if !exact_keys(
                r,
                &[
                    "kind",
                    "syntax_kind",
                    "start_byte",
                    "end_byte",
                    "start_row",
                    "start_column_byte",
                    "end_row",
                    "end_column_byte",
                ],
            ) {
                return false;
            }
            let (Some(start), Some(end)) = (r["start_byte"].as_u64(), r["end_byte"].as_u64())
            else {
                return false;
            };
            if start > end
                || start < route.byte_offset as u64
                || end > (route.byte_offset + route.source.len()) as u64
                || !matches!(r["kind"].as_str(), Some("ERROR" | "MISSING"))
                || !r["syntax_kind"].as_str().is_some_and(|s| {
                    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
                })
            {
                return false;
            }
            let (sr, sc) = point(&bytes, start as usize);
            let (er, ec) = point(&bytes, end as usize);
            r["start_row"] == sr
                && r["start_column_byte"] == sc
                && r["end_row"] == er
                && r["end_column_byte"] == ec
        })
    })
}

fn point(bytes: &[u8], index: usize) -> (usize, usize) {
    let prefix = &bytes[..index];
    (
        prefix.iter().filter(|b| **b == b'\n').count(),
        prefix.rsplit(|b| *b == b'\n').next().map_or(0, <[u8]>::len),
    )
}
fn exact_keys(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn safe_path(p: &str) -> bool {
    !p.is_empty()
        && !p.contains('\\')
        && !p.chars().any(char::is_control)
        && Path::new(p)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
pub(crate) fn identity(
    workspace: &str,
    path: &str,
    language: &str,
) -> (&'static str, &'static str, String) {
    if language == "python" {
        return (
            "python.ruff",
            "python_syntax_confirmation_needed",
            crate::python_syntax_confirmation::fingerprint(workspace, path),
        );
    }
    if matches!(language, "javascript" | "typescript" | "tsx") {
        return (
            "node.eslint.preparation",
            "eslint_prerequisites_require_review",
            crate::eslint_preparation::fingerprint(workspace, path),
        );
    }
    let mut hash = Sha256::new();
    for part in [
        "codeguard-native-syntax-confirmation-v1",
        workspace,
        path,
        language,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    (
        "syntax.native_confirmation",
        "native_syntax_confirmation_needed",
        format!("{:x}", hash.finalize()),
    )
}
