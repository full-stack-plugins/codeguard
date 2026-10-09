//! 统一check的有界Clang文档观察；显式档案不推断完整构建配置。
use crate::syntax_lint_arguments::SyntaxLintArguments;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
const FILE_LIMIT: usize = 64;
const REPORT_LIMIT: usize = 16 * 1024 * 1024;
fn tool_state(tool: Option<&Path>) -> Value {
    let resolved = tool.and_then(|p| p.canonicalize().ok());
    let sha = resolved
        .as_ref()
        .and_then(|p| read_bounded_regular_file(p, 256 * 1024 * 1024).ok())
        .map(|b| format!("{:x}", Sha256::digest(b)));
    json!({"selected":tool,"resolved":resolved,"sha256":sha})
}
/// 返回未执行的明确上下文；参数为语言、来源数量与选定工具/标准，不生成违规。
pub(crate) fn empty(
    language: &str,
    count: usize,
    tool: Option<&Path>,
    standard: Option<&str>,
) -> Value {
    json!({"schema_version":"0.1.0","report_type":"c_family_documentation_scan","language":language,"scope":"discovered_standalone_files","project_configuration":"unknown","standard":standard,"tool":tool_state(tool),"source_file_count":count,"unobserved_count":count,"file_limit":FILE_LIMIT,"report_limit_bytes":REPORT_LIMIT,"scope_stable":true,"local_scan_complete":false,"native_task_started":false,"status":"not_run","reason":"native_task_not_started","files":[],"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"})
}
/// 在任务图资源锁内观察最多64文件，累计反馈不超过16MiB，不执行项目脚本或猜标准。
pub(crate) fn observe(
    root: &Path,
    language: &str,
    paths: &BTreeSet<String>,
    tool: Option<&Path>,
    standard: Option<&str>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut r = empty(language, paths.len(), tool, standard);
    let (Some(tool), Some(standard)) = (tool, standard) else {
        r["status"] = json!("context_required");
        r["reason"] = json!("clang_documentation_context_required");
        return r;
    };
    let mut rows = Vec::new();
    let mut bytes = 0usize;
    for relative in paths.iter().take(FILE_LIMIT) {
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            r["reason"] = json!("request_cancelled");
            break;
        }
        if Instant::now() >= deadline {
            r["reason"] = json!("request_deadline_exceeded");
            break;
        }
        let request = SyntaxLintArguments {
            language: language.into(),
            source: root.join(relative),
            workspace: Some(root.to_path_buf()),
            clang_tool: Some(tool.to_path_buf()),
            standard: Some(standard.into()),
            json: true,
            timeout_ms: deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .clamp(1, u64::MAX as u128) as u64,
        };
        let feedback =
            crate::c_family_comments_command::observe(&request, deadline, cancelled, false).ok();
        let size = feedback
            .as_ref()
            .and_then(|v| serde_json::to_vec(v).ok())
            .map_or(0, |b| b.len());
        if size > REPORT_LIMIT.saturating_sub(bytes) {
            r["reason"] = json!("aggregate_report_limit");
            break;
        }
        bytes += size;
        rows.push(json!({"path":relative,"current":false,"reason":if feedback.is_some(){"native_documentation_observed"}else{"source_scope_unavailable"},"feedback":feedback}));
    }
    r["native_task_started"] = json!(
        rows.iter()
            .any(|f| f["feedback"]["native"]["version"].is_string())
    );
    r["unobserved_count"] = json!(paths.len().saturating_sub(rows.len()));
    r["files"] = json!(rows);
    r["status"] = json!("partial");
    if r["reason"] == "native_task_not_started" {
        r["reason"] = json!("native_documentation_coverage_unverified");
    }
    enforce_report_limit(&mut r);
    refresh(root, &mut r, deadline);
    r
}
/// 再次核对源码、原工具和共享预算；失稳撤回定位，不授权源码修复。
pub(crate) fn refresh(root: &Path, r: &mut Value, deadline: Instant) {
    let selected = r["tool"]["selected"].as_str().map(PathBuf::from);
    let stable = r["scope_stable"] == true
        && tool_state(selected.as_deref()) == r["tool"]
        && Instant::now() < deadline
        && !codeguard_runtime::sigint_cancellation_requested();
    if !stable {
        r["scope_stable"] = json!(false);
    }
    for row in r["files"].as_array_mut().into_iter().flatten() {
        let current = stable
            && row["path"]
                .as_str()
                .and_then(|p| crate::plain_syntax_source::read_plain_source(&root.join(p)).ok())
                .is_some_and(|b| {
                    row["feedback"]["source_sha256"] == format!("{:x}", Sha256::digest(b))
                });
        row["current"] = json!(current);
        if !current && row["feedback"].is_object() {
            let f = &mut row["feedback"];
            f["local_scan_complete"] = json!(false);
            f["native"]["status"] = json!("incomplete");
            f["native"]["reason"] = json!(if f["command_status"] == "cancelled" {
                "request_cancelled"
            } else {
                "clang_execution_incomplete"
            });
            f["reason"] = json!("native_documentation_coverage_unverified");
            f["native"]["diagnostics"] = json!([]);
            f["documentation_findings"] = json!([]);
            f["unclassified_native_diagnostics"] = json!([]);
            f["documentation_structure"] = json!({"status":"incomplete","reason":"clang_execution_incomplete","observation":null,"native_raw_diagnostic_count":null});
            f["next"] = Value::Null;
            for key in ["workbench", "structural_workbench"] {
                if f[key].is_object() {
                    f[key]["task_ids"] = json!([]);
                    f[key]["status"] = json!("incomplete");
                    f[key]["reason"] = json!("input_or_scope_changed");
                }
            }
            if f.get("structural_next").is_some() {
                f["structural_next"] = Value::Null;
            }
            row["reason"] = json!("input_or_scope_changed");
        }
    }
    r["local_scan_complete"] = json!(
        stable
            && r["status"] == "partial"
            && r["unobserved_count"] == 0
            && r["files"].as_array().is_some_and(|rows| !rows.is_empty()
                && rows
                    .iter()
                    .all(|f| f["current"] == true && f["feedback"]["local_scan_complete"] == true))
    );
}
/// 在全项目范围复核后连接已有工作台；共享截止、取消和原工具失稳时不持久化。
pub(crate) fn connect(root: &Path, r: &mut Value, deadline: Instant) {
    refresh(root, r, deadline);
    if r["scope_stable"] != true || r["status"] != "partial" {
        return;
    }
    let Some(tool) = r["tool"]["selected"].as_str().map(PathBuf::from) else {
        return;
    };
    r["task_status"] = json!("partial");
    for row in r["files"].as_array_mut().into_iter().flatten() {
        if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
            break;
        }
        if row["current"] != true || !row["feedback"].is_object() {
            continue;
        }
        let f = &mut row["feedback"];
        crate::c_family_comments_workbench::connect(root, &tool, f, deadline);
        crate::c_family_structure_workbench::connect(root, &tool, f, deadline);
        if f.get("workbench").is_some() {
            f["schema_version"] = json!("0.9.0");
        }
    }
    enforce_report_limit(r);
    refresh(root, r, deadline);
}

// 累计预算包含工作台/下一步投影；丢弃超额尾部，只保留有界局部反馈。
fn enforce_report_limit(r: &mut Value) {
    let mut bytes = serde_json::to_vec(r).map_or(usize::MAX, |b| b.len());
    let mut dropped = false;
    while bytes > REPORT_LIMIT.saturating_sub(4096) {
        let Some(row) = r["files"].as_array_mut().and_then(Vec::pop) else {
            break;
        };
        bytes = bytes.saturating_sub(serde_json::to_vec(&row).map_or(0, |b| b.len()));
        dropped = true;
    }
    if dropped {
        r["reason"] = json!("aggregate_report_limit");
        r["local_scan_complete"] = json!(false);
        r["unobserved_count"] = json!(
            r["source_file_count"]
                .as_u64()
                .unwrap_or(0)
                .saturating_sub(r["files"].as_array().map_or(0, Vec::len) as u64)
        );
    }
}
