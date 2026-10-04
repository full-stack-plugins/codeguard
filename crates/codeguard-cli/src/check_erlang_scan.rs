//! 统一检查中的有界 Erlang 原生 forms 观察；不执行源码或预处理器。

use crate::{erlang_tool_selection::ErlangToolSelection, plain_syntax_source::read_plain_source};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const MAX_FILES: usize = 64;

/// 对已发现的工作区相对源码执行原生 forms 检查。
/// 参数为项目根、冻结路径集、显式工具及共享预算/取消状态；返回局部观察，不签发项目通过。
pub(crate) fn observe(
    root: &Path,
    paths: &BTreeSet<String>,
    tool: Option<PathBuf>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let selection = ErlangToolSelection::discover(tool);
    let mut report = json!({
        "schema_version":"0.1.0", "report_type":"erlang_forms_scan",
        "tool_selection":selection.report(), "scope":"single_file_forms_without_preprocessing",
        "authority":"local_unverified", "coverage_proven":false, "delivery_decision":"not_evaluated",
        "local_forms_complete":false, "scope_stable":true, "reason":"erlang_native_scan_incomplete",
        "source_file_count":paths.len(), "unobserved_count":paths.len().saturating_sub(MAX_FILES),
        "files":[], "task_status":"native_findings_not_integrated", "task_id":null
    });
    let mut files = Vec::new();
    for relative in paths.iter().take(MAX_FILES) {
        let mut file = json!({"path":relative,"source_sha256":null,"current":false,
            "native":unavailable("not_run", "erlang_tool_not_found_on_path"),
            "findings":[],"recheck_argv":null,
            "next_action":"读取候选初检；疑似异常必须准备 OTP 28 原生工具确认，初检正常时推荐准备原生工具；完整项目 lint、编译和测试仍需执行"});
        let reason = if cancelled.load(Ordering::Relaxed)
            || codeguard_runtime::sigint_cancellation_requested()
        {
            Some("request_cancelled")
        } else if Instant::now() >= deadline {
            Some("request_deadline_exceeded")
        } else {
            None
        };
        if let Some(reason) = reason {
            file["native"] = unavailable("incomplete", reason);
            file["next_action"] =
                json!("恢复本轮检查预算或取消条件后重跑；不要把未观察源码当作通过");
            files.push(file);
            continue;
        }
        let source = match read_plain_source(&root.join(relative)) {
            Ok(source) => source,
            Err(reason) => {
                file["native"] = unavailable("incomplete", reason);
                file["next_action"] =
                    json!("恢复普通 UTF-8 源码路径和读取预算后重跑；不要据此认定源码违规");
                files.push(file);
                continue;
            }
        };
        file["source_sha256"] = json!(digest(&source));
        file["current"] = json!(true);
        if let Some(tool) = selection.tool() {
            file["native"] = crate::erlang_syntax_probe::observe(tool, &source, deadline);
            let native = file["native"].clone();
            file["findings"] = native["diagnostics"].clone();
            file["next_action"] = json!(match native["status"].as_str() {
                Some("diagnostics_observed") =>
                    "按当前原生语法位置修复并执行 recheck_argv；仍需项目完整 lint、预处理、编译和测试",
                Some("completed") =>
                    "本文件原生 forms 无诊断；继续项目完整 lint、预处理、编译和测试，不能据此关闭任务或交付",
                _ if native["preprocessing_unresolved"] == true =>
                    "使用项目的完整预处理和编译链核对宏/条件编译；不要按未展开的宏诊断修改源码",
                _ => "先恢复所选工具版本、执行预算或环境，再重跑同一原生检查；不要反复修改无关源码",
            });
            if matches!(
                native["status"].as_str(),
                Some("completed" | "diagnostics_observed")
            ) {
                file["recheck_argv"] = json!([
                    "codeguard",
                    "lint",
                    "erlang",
                    root.join(relative),
                    "--erl-tool",
                    tool,
                    "--format=json"
                ]);
            }
        }
        if read_plain_source(&root.join(relative)).ok().as_deref() != Some(source.as_slice()) {
            invalidate_file(&mut file, "erlang_source_changed_during_check");
        }
        files.push(file);
    }
    report["files"] = json!(files);
    summarize(&mut report);
    // 后续文件可能改变先前工具或源码；整个原生阶段结束后再核对一次，不复用过期指引。
    refresh(root, &mut report, deadline);
    report
}

/// 复核原生阶段结束后的源码/工具身份，失效时撤回当前定位和可复用指令。
/// 不重新运行原生工具，预算不足保持未完成。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let tool = report["tool_selection"]["executable"]
        .as_str()
        .map(PathBuf::from);
    let current_tool_sha = tool.as_deref().and_then(|path| {
        if Instant::now() >= deadline {
            None
        } else {
            read_bounded_regular_file(path, 64 * 1024 * 1024)
                .ok()
                .map(|bytes| digest(&bytes))
        }
    });
    for file in report["files"].as_array_mut().into_iter().flatten() {
        if file["current"] != true {
            continue;
        }
        let reason = if Instant::now() >= deadline {
            Some("request_deadline_exceeded")
        } else if file["native"]["tool_sha256"]
            .as_str()
            .is_some_and(|expected| current_tool_sha.as_deref() != Some(expected))
        {
            Some("erlang_tool_changed_after_check")
        } else if file["path"]
            .as_str()
            .and_then(|path| read_plain_source(&root.join(path)).ok())
            .is_none_or(|bytes| Some(digest(&bytes).as_str()) != file["source_sha256"].as_str())
        {
            Some("erlang_source_changed_after_check")
        } else {
            None
        };
        if let Some(reason) = reason {
            invalidate_file(file, reason);
        }
    }
    summarize(report);
}

/// 项目观察范围变化时撤回范围完整性，不删除仍与当前字节匹配的单文件发现。
/// 单文件位置和工具是否有效由 refresh 独立复核。
pub(crate) fn invalidate_scope(report: &mut Value) {
    report["scope_stable"] = json!(false);
    summarize(report);
}

/// 提取同轮、完整、非预处理 forms 覆盖的源码摘要；仅供候选阶段避免重复解析。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn covered_sources(report: &Value) -> std::collections::BTreeMap<String, String> {
    report["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|file| {
            file["current"] == true
                && matches!(
                    file["native"]["status"].as_str(),
                    Some("completed" | "diagnostics_observed")
                )
                && file["native"]["diagnostics_truncated"] == false
                && file["native"]["preprocessing_unresolved"] == false
        })
        .filter_map(|file| {
            Some((
                file["path"].as_str()?.to_owned(),
                file["source_sha256"].as_str()?.to_owned(),
            ))
        })
        .collect()
}

fn unavailable(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"version":null,"tool_sha256":null,
        "diagnostics":[],"diagnostics_truncated":false,"preprocessing_unresolved":false})
}

fn invalidate_file(file: &mut Value, reason: &str) {
    file["current"] = json!(false);
    file["native"]["status"] = json!("incomplete");
    file["native"]["reason"] = json!(reason);
    file["native"]["diagnostics"] = json!([]);
    file["findings"] = json!([]);
    file["recheck_argv"] = Value::Null;
    file["next_action"] = json!("原观察身份已失效；对当前源码与工具重新检查，不使用旧诊断修改源码");
}

fn summarize(report: &mut Value) {
    let complete = report["scope_stable"] == true
        && report["unobserved_count"] == 0
        && report["files"].as_array().is_some_and(|files| {
            !files.is_empty()
                && files.iter().all(|file| {
                    file["current"] == true
                        && matches!(
                            file["native"]["status"].as_str(),
                            Some("completed" | "diagnostics_observed")
                        )
                })
        });
    report["local_forms_complete"] = json!(complete);
    report["reason"] = json!(if complete {
        "erlang_forms_observed_unverified_project_coverage"
    } else {
        "erlang_native_scan_incomplete"
    });
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
