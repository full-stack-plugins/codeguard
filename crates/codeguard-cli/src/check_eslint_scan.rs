//! 聚合检查的 ESLint 原生阶段；复用单文件适配器，持久化留在串行汇总阶段。
use crate::discovery::DiscoveryReport;
use crate::eslint_lint_arguments::EslintLintArguments;
use crate::eslint_native_first_candidate::{node_on_path, observed_candidate_within};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 本轮聚合原生反馈及待串行同步的报告；不以本地观察授予交付权限。
pub(crate) struct CheckEslintScan {
    pub(crate) feedback: Value,
    reports: Vec<Value>,
}

/// 从本轮发现取 JS/TS 方言文件，去重并保持稳定路径顺序。
pub(crate) fn sources(discovery: &DiscoveryReport) -> BTreeSet<String> {
    discovery
        .languages
        .values()
        .flat_map(|item| item.source_files.iter())
        .filter(|path| {
            Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext,
                        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
                    )
                })
        })
        .cloned()
        .collect()
}

impl CheckEslintScan {
    /// 在共同截止时间内检查已发现文件；Node 显式路径优先，否则观察 PATH。
    /// 原生故障不吞掉兄弟文件结果，返回的准备报告不能误标为源码违规。
    pub(crate) fn run(
        root: &Path,
        paths: &BTreeSet<String>,
        node: Option<&Path>,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Self {
        let mut files = Vec::new();
        let mut reports = Vec::new();
        let node = node.map(Path::to_path_buf).or_else(node_on_path);
        let mut reason = "project_context_and_policy_unverified";
        for relative in paths.iter().take(10_000) {
            if cancelled.load(Ordering::Relaxed)
                || codeguard_runtime::sigint_cancellation_requested()
            {
                reason = "request_cancelled";
                break;
            }
            if Instant::now() >= deadline {
                reason = "request_deadline_exceeded";
                break;
            }
            let source = root.join(relative);
            let mut args = EslintLintArguments {
                source: source.clone(),
                node: node.clone(),
                entry: None,
                config: None,
                config_map: None,
                cwd: None,
                workspace: None,
                version: None,
                json: true,
                timeout_ms: 0,
            };
            let prepared = match observed_candidate_within(&source, root) {
                Ok(Some(candidate)) => {
                    args.entry = Some(candidate.entry);
                    args.config = Some(candidate.config);
                    args.cwd = Some(candidate.root);
                    args.version = Some(candidate.version);
                    if node.is_some() {
                        Ok(())
                    } else {
                        Err("eslint_node_runtime_unresolved")
                    }
                }
                Ok(None) => Err("eslint_execution_context_missing"),
                Err(reason) => Err(reason),
            };
            let before = read_bounded_regular_file(&source, 16 * 1024 * 1024).ok();
            let (feedback, mut report) = match prepared {
                Ok(()) => crate::eslint_lint_command::observe_for_task(root, &args, deadline),
                Err("eslint_execution_context_missing") => (
                    crate::eslint_lint_command::feedback("eslint_execution_context_missing"),
                    None,
                ),
                Err(reason) => (
                    crate::eslint_lint_command::feedback(reason),
                    crate::eslint_preparation::prepare(root, &args, reason).ok(),
                ),
            };
            if report.is_none() {
                if let Some(reason) = feedback["reason"].as_str().filter(|r| {
                    *r != "eslint_execution_context_missing"
                        && crate::eslint_preparation::valid_reason(r)
                }) {
                    report = crate::eslint_preparation::prepare(root, &args, reason).ok();
                }
            }
            let source_sha256 = before
                .filter(|bytes| {
                    feedback["local_coherent"] == true
                        && read_bounded_regular_file(&source, 16 * 1024 * 1024)
                            .ok()
                            .as_ref()
                            == Some(bytes)
                })
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
            if let Some(report) = report {
                reports.push(report);
            }
            files.push(json!({"path":relative,"source_sha256":source_sha256,"feedback":feedback}));
        }
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            reason = "request_cancelled";
        } else if Instant::now() >= deadline {
            reason = "request_deadline_exceeded";
        }
        let complete = files.len() == paths.len()
            && !files.is_empty()
            && files.iter().all(|row| row["source_sha256"].is_string());
        if !complete && reason == "project_context_and_policy_unverified" {
            reason = "eslint_files_incomplete";
        }
        let unexecuted: Vec<_> = paths.iter().skip(files.len()).collect();
        Self {
            feedback: json!({
                "status":if complete {"local_observation"} else {"incomplete"},"reason":reason,
                "coverage_proven":false,"delivery_decision":"not_evaluated","files":files,
                "unexecuted_files":unexecuted,"backlog_status":"not_connected","new_findings":0,"new_blockers":0,"next":null
            }),
            reports,
        }
    }

    /// 在任务图执行结束后串行入队和同步，以免多个原生任务争抢工作台锁。
    /// 未初始化工作区保持只输出，截止时间用尽则保留明确的未同步状态。
    pub(crate) fn sync(&mut self, root: &Path, deadline: Instant) {
        if self.reports.is_empty() {
            return;
        }
        for report in &self.reports {
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                self.feedback["backlog_status"] = json!("deadline_or_cancelled");
                return;
            }
            if crate::work_sync::save_local_report(root, report).is_err() {
                self.feedback["backlog_status"] = json!("backlog_update_failed");
                return;
            }
        }
        match crate::work_sync::sync_local_workspace(root) {
            Ok(summary) => {
                self.feedback["backlog_status"] = json!(if summary.failed_reports == 0 {
                    "synced_partial"
                } else {
                    "sync_incomplete"
                });
                self.feedback["new_findings"] = json!(summary.new_findings);
                self.feedback["new_blockers"] = json!(summary.new_blockers);
                self.feedback["next"] =
                    crate::next_command::read_local_brief_for_checker(root, "node.eslint")
                        .ok()
                        .filter(|value| !value.is_null() && !value["repair_brief"].is_null())
                        .or_else(|| {
                            crate::next_command::read_local_brief_for_checker(
                                root,
                                "node.eslint.preparation",
                            )
                            .ok()
                        })
                        .unwrap_or(Value::Null);
            }
            Err(_) => self.feedback["backlog_status"] = json!("backlog_update_failed"),
        }
    }
}

/// 仅相同路径和字节的完整原生结果可免去重复 WASM；忽略、fatal、故障和未执行文件不能免检。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn covers(report: &Value, relative: &str, source: &[u8]) -> bool {
    let digest = format!("{:x}", Sha256::digest(source));
    report["files"].as_array().is_some_and(|files| {
        files.iter().any(|row| {
            row["path"] == relative
                && row["source_sha256"] == digest
                && row["feedback"]["local_coherent"] == true
        })
    })
}

#[cfg(all(test, feature = "wasm-precheck"))]
mod tests {
    use super::covers;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    #[test]
    fn only_same_source_and_complete_native_result_suppress_repeated_wasm() {
        let source = b"debugger;";
        let mut report = json!({"files":[{"path":"app.js", "source_sha256":format!("{:x}",Sha256::digest(source)),"feedback":{"local_coherent":true}}]});
        assert!(covers(&report, "app.js", source));
        assert!(!covers(&report, "other.js", source));
        assert!(!covers(&report, "app.js", b"changed"));
        report["files"][0]["feedback"]["local_coherent"] = json!(false);
        assert!(!covers(&report, "app.js", source));
    }
}
