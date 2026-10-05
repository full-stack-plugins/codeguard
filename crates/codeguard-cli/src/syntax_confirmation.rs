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
                && row["structural_observation_count"].as_u64().unwrap_or(0) == 0
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
            let schema_version = if language == "cfquery" {
                "0.11.0"
            } else if rows
                .iter()
                .any(|row| row.get("structural_observations").is_some())
            {
                if language == "go" { "0.8.0" } else { "0.7.0" }
            } else if rows.iter().any(|row| row["recovery_count"] == 0) {
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
        Some("0.2.0" | "0.4.0" | "0.5.0" | "0.6.0" | "0.9.0" | "0.10.0")
    ) {
        return crate::native_syntax_confirmation::valid_history_report(root, workspace, report)
            && crate::syntax_task_recheck::inputs_current(root, &report["native_evidence"]);
    }
    let Some(path) = report["scope"].as_str().filter(|p| safe_path(p)) else {
        return false;
    };
    let source = root.join(path);
    if source.canonicalize().ok().as_deref() != Some(source.as_path()) {
        return false;
    }
    let Ok(bytes) = read_bounded_regular_file(&source, 1024 * 1024) else {
        return false;
    };
    valid_source_snapshot(workspace, report, &bytes)
}

/// 对指定冻结字节核对候选报告身份、固定资产和位置，不读取修复后的当前文件。
/// 参数为工作区、历史报告和原始源码；返回内部一致性，调用者仍须核对首次收据和批准来源。
pub(crate) fn valid_source_snapshot(workspace: &str, report: &Value, bytes: &[u8]) -> bool {
    if bytes.len() > 1024 * 1024 || std::str::from_utf8(bytes).is_err() {
        return false;
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
    if !matches!(
        report["schema_version"].as_str(),
        Some("0.1.0" | "0.3.0" | "0.7.0" | "0.8.0" | "0.11.0")
    ) || (report["schema_version"] == "0.7.0" && language != "python")
        || (report["schema_version"] == "0.8.0" && language != "go")
        || (report["schema_version"] == "0.11.0" && language != "cfquery")
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
    let sha = format!("{:x}", Sha256::digest(bytes));
    let Some(grammar) = bundled_grammar_candidates()
        .ok()
        .and_then(|m| m.assets.into_iter().find(|a| a.language == language))
    else {
        return false;
    };
    let routes = crate::grammar_route::route_source(path, bytes);
    let Some(rows) = report["observations"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 64)
    else {
        return false;
    };
    let mut offsets = std::collections::BTreeSet::new();
    rows.iter().all(|row| {
        let structural = matches!(report["schema_version"].as_str(), Some("0.7.0" | "0.8.0"))
            || (report["schema_version"] == "0.11.0"
                && row.get("structural_observations").is_some());
        let mut row_keys = vec![
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
        ];
        if report["schema_version"] == "0.11.0" {
            row_keys.push("fragment_source_sha256");
        }
        if structural {
            row_keys.extend(["structural_observation_count", "structural_observations"]);
        }
        if !exact_keys(row, &row_keys)
            || row["path"] != path
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
        if report["schema_version"] == "0.11.0"
            && row["fragment_source_sha256"] != format!("{:x}", Sha256::digest(route.source))
        {
            return false;
        }
        if !offsets.insert(route.byte_offset) {
            return false;
        }
        if structural {
            let Some(count) = row["structural_observation_count"]
                .as_u64()
                .filter(|count| *count > 0 && *count <= 128 && (language != "go" || *count == 1))
            else {
                return false;
            };
            let Some(structures) = row["structural_observations"]
                .as_array()
                .filter(|rows| rows.len() == (count as usize).min(8))
            else {
                return false;
            };
            if (language != "cfquery" && (route.byte_offset != 0 || route.source != bytes))
                || !structures.iter().all(|value| {
                    serde_json::from_value::<crate::syntax_worker_structure::SyntaxWorkerStructure>(
                        value.clone(),
                    )
                    .is_ok_and(|value| {
                        value.valid(language, bytes)
                            && value.start_byte >= route.byte_offset
                            && value.end_byte <= route.byte_offset + route.source.len()
                    })
                })
            {
                return false;
            }
        }
        let Some(count) = row["recovery_count"].as_u64().filter(|c| {
            *c <= 4096
                && (!structural
                    || *c + row["structural_observation_count"].as_u64().unwrap_or(129) <= 128)
                && (*c > 0
                    || structural
                    || (matches!(report["schema_version"].as_str(), Some("0.3.0" | "0.11.0"))
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
            let (sr, sc) = point(bytes, start as usize);
            let (er, ec) = point(bytes, end as usize);
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

#[cfg(test)]
mod snapshot_tests {
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    fn report(source: &[u8], structural: bool) -> Value {
        let workspace = "generic-python-snapshot";
        let scope = "sample.py";
        let (checker, reason, fingerprint) = super::identity(workspace, scope, "python");
        let (grammar, _) = codeguard_adapters::bundled_grammar_candidate("python").unwrap();
        let mut row = json!({"path":scope,"language":"python","scope":"whole_file","byte_offset":0,
            "status":"candidate_observed","reason":null,"grammar_qualified":false,
            "source_sha256":format!("{:x}", Sha256::digest(source)),"grammar_sha256":grammar.sha256,
            "recovery_count":1,"recoveries":[{"kind":"ERROR","syntax_kind":"ERROR",
                "start_byte":0,"end_byte":3,"start_row":0,"start_column_byte":0,"end_row":0,"end_column_byte":3}],
            "known_limitations":grammar.known_limitations});
        if structural {
            row["recovery_count"] = json!(0);
            row["recoveries"] = json!([]);
            row["structural_observation_count"] = json!(1);
            row["structural_observations"] = json!([{"basis":"codeguard_structure_rule",
                "rule_id":"codeguard.python.required_suite","rule_version":"1.0.0",
                "rule_sha256":codeguard_adapters::python_suite_rule_sha256(),
                "parent_syntax_kind":"function_definition","start_byte":14,"end_byte":14,
                "start_row":1,"start_column_byte":0,"end_row":1,"end_column_byte":0}]);
        }
        json!({"schema_version":if structural {"0.7.0"} else {"0.1.0"},
            "report_type":"syntax_confirmation_observation","workspace_binding":"bound",
            "workspace_id":workspace,"run_id":"syntax-confirm-1-1","authority":"local_unverified",
            "coverage_proven":false,"delivery_decision":"not_evaluated","execution":"incomplete",
            "checker_id":checker,"reason_code":reason,"blocker_id":format!("CG-B-{}", &fingerprint[..32]),
            "fingerprint":fingerprint,"build_root":".","scope":scope,"language":"python",
            "affected_paths":[scope],"observations":[row]})
    }
    #[test]
    fn original_generic_snapshot_is_not_replaced_by_repaired_source() {
        let source = b"def broken():\n";
        for structural in [false, true] {
            let mut value = report(source, structural);
            assert!(super::valid_source_snapshot(
                "generic-python-snapshot",
                &value,
                source
            ));
            assert!(!super::valid_source_snapshot(
                "generic-python-snapshot",
                &value,
                b"def broken():\n    pass\n"
            ));
            assert!(!super::valid_source_snapshot(
                "other-workspace",
                &value,
                source
            ));
            value["observations"][0]["grammar_sha256"] = json!("a".repeat(64));
            assert!(!super::valid_source_snapshot(
                "generic-python-snapshot",
                &value,
                source
            ));
        }
    }
    #[test]
    fn generic_snapshot_rejects_undecodable_source_and_forged_coordinates() {
        let bad = b"value = \xff\n";
        assert!(!super::valid_source_snapshot(
            "generic-python-snapshot",
            &report(bad, false),
            bad
        ));
        let source = b"def broken():\n";
        let mut value = report(source, false);
        value["observations"][0]["recoveries"][0]["end_column_byte"] = json!(999);
        assert!(!super::valid_source_snapshot(
            "generic-python-snapshot",
            &value,
            source
        ));
    }
}
