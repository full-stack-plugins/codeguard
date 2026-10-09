//! Go编辑原生语法观察；固定SDK的gofmt只读取选中文件冻结stdin，不运行go vet或源码。
use crate::{go_tool_selection::GoToolSelection, plain_syntax_source::read_plain_source};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 观察限定相对路径；共享截止时间和取消状态，最多64文件，不运行项目脚本。
pub(crate) fn observe(
    root: &Path,
    paths: &BTreeSet<String>,
    tool: Option<PathBuf>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let selection = GoToolSelection::discover(tool);
    let mut report = json!({"schema_version":"0.1.0","report_type":"go_syntax_scan","scope":"single_frozen_go_files","tool_selection":selection.report(),"source_file_count":paths.len(),"unobserved_count":paths.len().saturating_sub(64),"scope_stable":true,"local_parse_complete":false,"files":[],"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    let mut files = Vec::new();
    for relative in paths.iter().take(64) {
        let mut file = json!({"path":relative,"source_sha256":null,"current":false,"tool_path":selection.tool().and_then(|p|p.canonicalize().ok()),"native":crate::go_syntax_probe::unavailable("go_syntax_tool_not_provided"),"task_id":null,"task_sync_reason":null,"recheck_argv":null,"next_action":"读取候选初检；疑似异常需要Go SDK确认，完整零恢复时推荐准备原生lint"});
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            file["native"] = crate::go_syntax_probe::unavailable("request_cancelled");
            files.push(file);
            continue;
        }
        if Instant::now() >= deadline {
            file["native"] = crate::go_syntax_probe::unavailable("request_deadline_exceeded");
            files.push(file);
            continue;
        }
        let source = match read_plain_source(&root.join(relative)) {
            Ok(s) => s,
            Err(_) => {
                file["native"] =
                    crate::go_syntax_probe::unavailable("go_source_unavailable_or_invalid");
                files.push(file);
                continue;
            }
        };
        file["source_sha256"] = json!(digest(&source));
        file["current"] = json!(true);
        if let Some(tool) = selection.tool() {
            file["native"] = crate::go_project_version::observe(
                root,
                &root.join(relative),
                tool,
                &source,
                deadline,
                cancelled,
            );
            let has_syntax = file["native"]["diagnostics"]
                .as_array()
                .is_some_and(|r| !r.is_empty());
            file["next_action"] = json!(if has_syntax {
                "先核对项目Go版本是否适用，再按当前原生行号确认和修复语法；按UTF-8字节列定位；继续go vet与完整项目检查"
            } else if file["native"]["status"] == "completed" {
                "本文件原生零诊断；继续项目 lint、类型、注释、安全与依赖检查，不凭此关闭任务"
            } else {
                "核对项目声明版本、选定Go1.23.4及同目录gofmt及预算后原工具复检；不改无关源码、不换工具绕过失败"
            });
        }
        files.push(file);
    }
    report["files"] = json!(files);
    refresh(root, &mut report, deadline);
    report
}
/// 复核每份原生观察的源码、入口目标和 launcher 字节；失效时撤回所有定位。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let selected = report["tool_selection"]["executable"]
        .as_str()
        .map(PathBuf::from);
    let resolved = selected.as_ref().and_then(|p| p.canonicalize().ok());
    let sha = resolved
        .as_ref()
        .and_then(|p| codeguard_runtime::read_bounded_regular_file(p, 64 * 1024 * 1024).ok())
        .map(|b| digest(&b));
    for file in report["files"].as_array_mut().into_iter().flatten() {
        if file["current"] != true {
            continue;
        }
        let reason = if Instant::now() >= deadline {
            Some("request_deadline_exceeded")
        } else if matches!(
            file["native"]["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) && file["path"]
            .as_str()
            .is_none_or(|p| !crate::go_project_version::compatible(root, &root.join(p)))
        {
            Some("go_project_version_changed_after_check")
        } else if file["tool_path"]
            .as_str()
            .is_some_and(|p| resolved.as_deref() != Some(Path::new(p)))
            || file["native"]["tool_sha256"]
                .as_str()
                .is_some_and(|s| sha.as_deref() != Some(s))
        {
            Some("go_tool_changed_after_check")
        } else if matches!(
            file["native"]["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) && selected
            .as_ref()
            .is_none_or(|p| !crate::go_syntax_probe::companion_current(p, &file["native"]))
        {
            Some("go_companion_changed_after_check")
        } else if file["path"]
            .as_str()
            .and_then(|p| read_plain_source(&root.join(p)).ok())
            .is_none_or(|b| file["source_sha256"] != digest(&b))
        {
            Some("go_source_changed_after_check")
        } else {
            None
        };
        if let Some(reason) = reason {
            file["current"] = json!(false);
            file["native"] = crate::go_syntax_probe::unavailable(reason);
            file["recheck_argv"] = Value::Null;
            file["next_action"] = json!("原观察已失效；先复检当前输入，不沿用旧位置修补源码");
        }
    }
    for file in report["files"].as_array_mut().into_iter().flatten() {
        if file["current"] == true {
            if let (Some(id), Some(tool)) = (file["task_id"].as_str(), selected.as_ref()) {
                file["recheck_argv"] = json!([
                    "codeguard",
                    "task",
                    "verify",
                    id,
                    root,
                    "--go-tool",
                    tool,
                    "--format=json"
                ]);
            }
        }
    }
    report["local_parse_complete"] = json!(
        report["scope_stable"] == true
            && report["unobserved_count"] == 0
            && report["files"].as_array().is_some_and(|r| !r.is_empty()
                && r.iter().all(|f| f["current"] == true
                    && matches!(
                        f["native"]["status"].as_str(),
                        Some("completed" | "diagnostics_observed")
                    )))
    );
}
/// 已选择原生入口的文件不再改走 WASM；该偏好不证明原生检查成功或覆盖完整。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn prefers(report: &Value, relative: &str) -> bool {
    report["tool_selection"]["source"]
        .as_str()
        .is_some_and(|s| s != "not_found")
        && relative.ends_with(".go")
        && (report["files"]
            .as_array()
            .is_some_and(|r| r.iter().any(|f| f["path"] == relative))
            || report["unobserved_count"].as_u64().is_some_and(|n| n > 0))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
