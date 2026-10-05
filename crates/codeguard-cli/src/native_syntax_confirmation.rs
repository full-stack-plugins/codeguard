//! 原生 forms 首次证据复用稳定确认任务；不伪造 WASM 观察或授予关闭权威。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Component, Path},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// 为单文件入口定位最近已有工作台并复用同一原生证据连接器。
/// 参数为已读取的源码路径、当前原生反馈与截止时间；不初始化目录或跨越损坏工作台。
pub(crate) fn connect_file(source: &Path, report: &mut Value, deadline: Instant) {
    let Ok(absolute) = source.canonicalize() else {
        return;
    };
    let Some(root) = absolute
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .find(|p| std::fs::symlink_metadata(p.join(".codeguard")).is_ok())
    else {
        return;
    };
    let Some(relative) = absolute.strip_prefix(root).ok().and_then(Path::to_str) else {
        return;
    };
    let mut scan = json!({"report_type":if report["language"] == "zig" {"zig_ast_scan"}else if report["language"] == "ruby" {"ruby_syntax_scan"}else{"erlang_forms_scan"},"tool_selection":report["tool_selection"],"files":[{
        "path":relative,"source_sha256":report["source_sha256"],"current":true,"tool_path":report["tool_selection"]["executable"].as_str().and_then(|p| Path::new(p).canonicalize().ok()),"native":report["native"]}]});
    connect(root, &mut scan, deadline);
    report["schema_version"] = json!("0.3.0");
    report["workspace_binding"] = json!(if scan["schema_version"] == "0.2.0" {
        "bound"
    } else {
        "invalid"
    });
    report["task_id"] = scan["files"][0]["task_id"].clone();
    report["task_status"] = scan["task_status"]
        .as_str()
        .map_or(json!("incomplete"), |s| json!(s));
    report["task_sync_reason"] = if scan["schema_version"] == "0.2.0" {
        scan["files"][0]["task_sync_reason"].clone()
    } else {
        json!("workspace_invalid")
    };
}

/// 将同轮当前 Erlang 原生文件观察连接到已初始化工作台；不重复执行检查器。
/// 返回逐文件真实任务引用，持久化失败保留具体原因，不影响原生诊断。
pub(crate) fn connect(root: &Path, scan: &mut Value, deadline: Instant) {
    let Some(workspace) = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
    else {
        return;
    };
    let language = if scan["report_type"] == "zig_ast_scan" {
        "zig"
    } else if scan["report_type"] == "ruby_syntax_scan" {
        "ruby"
    } else if scan["report_type"] == "swift_parse_scan" {
        "swift"
    } else if scan["report_type"] == "kotlin_compile_scan" {
        "kotlin"
    } else {
        "erlang"
    };
    scan["schema_version"] = json!("0.2.0");
    let tool = scan["tool_selection"]["executable"].clone();
    let mut failed = 0;
    let mut pending = Vec::new();
    for (index, file) in scan["files"]
        .as_array_mut()
        .into_iter()
        .flatten()
        .enumerate()
    {
        file["task_id"] = Value::Null;
        file["task_sync_reason"] = Value::Null;
        if file["current"] != true {
            failed += 1;
            file["task_sync_reason"] = json!("native_confirmation_source_not_current");
            continue;
        }
        let result = (|| {
            let path = file["path"]
                .as_str()
                .ok_or("native_confirmation_scope_invalid")?;
            let (checker, reason, fingerprint) =
                crate::syntax_confirmation::identity(&workspace, path, language);
            let id = format!("CG-B-{}", &fingerprint[..32]);
            // 新的原生零诊断不创建待办；已有任务仍保存当前观察，不据此关闭。
            if (file["native"]["status"] == "completed"
                || (matches!(language, "kotlin" | "swift" | "zig" | "ruby")
                    && cfg!(feature = "wasm-precheck")
                    && matches!(
                        file["native"]["reason"].as_str(),
                        Some(
                            "kotlin_tool_not_found"
                                | "swift_tool_not_found"
                                | "zig_tool_not_found"
                                | "ruby_tool_not_found"
                        )
                    )))
                && !root
                    .join(format!(".codeguard/findings/{id}/finding.json"))
                    .exists()
            {
                return Ok(None);
            }
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                return Err("deadline_or_cancelled");
            }
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "clock_unavailable")?
                .as_nanos();
            let report = json!({"schema_version":if language=="ruby" {"0.9.0"}else if language=="zig" {"0.6.0"}else if language=="swift" {"0.5.0"}else if language=="kotlin" {"0.4.0"}else{"0.2.0"},"report_type":"syntax_confirmation_observation",
                "workspace_binding":"bound","workspace_id":workspace,"run_id":format!("syntax-confirm-{}-{nanos}",std::process::id()),
                "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","execution":"incomplete",
                "checker_id":checker,"reason_code":reason,"blocker_id":id,"fingerprint":fingerprint,
                "build_root":".","scope":path,"language":language,"affected_paths":[path],"observations":[],
                "native_evidence":{"target":{"path":path,"language":language,"source_sha256":file["source_sha256"]},
                    "tool_path":if matches!(language,"kotlin"|"swift"|"zig"|"ruby") {file["tool_path"].clone()}else{tool.clone()},"native":file["native"]}});
            if !valid_history_report(root, &workspace, &report)
                || !crate::syntax_task_recheck::inputs_current(root, &report["native_evidence"])
            {
                return Err("native_confirmation_inputs_invalid");
            }
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                return Err("deadline_or_cancelled");
            }
            crate::work_sync::save_local_report(root, &report)?;
            Ok(Some((
                id,
                report["run_id"]
                    .as_str()
                    .ok_or("report_run_id_invalid")?
                    .to_owned(),
            )))
        })();
        match result {
            Ok(Some((id, run))) => pending.push((index, id, run)),
            Ok(None) => {}
            Err(reason) => {
                failed += 1;
                file["task_sync_reason"] = json!(reason);
            }
        }
    }
    // 同轮最多 64 文件只同步一次；不为每个文件重复扫描整个历史目录。
    if !pending.is_empty() {
        let sync =
            if Instant::now() >= deadline || codeguard_runtime::sigint_cancellation_requested() {
                Err("deadline_or_cancelled")
            } else {
                crate::work_sync::sync_local_workspace(root).and_then(|s| {
                    if s.failed_reports == 0 {
                        Ok(())
                    } else {
                        Err("native_confirmation_sync_incomplete")
                    }
                })
            };
        for (index, id, run) in pending {
            let file = &mut scan["files"][index];
            let result = sync.and_then(|()| {
                if root.join(format!(".codeguard/tasks/{id}.md")).is_file()
                    && root
                        .join(format!(".codeguard/state/consumed/{run}.json"))
                        .is_file()
                {
                    Ok(())
                } else {
                    Err("native_confirmation_task_missing")
                }
            });
            match result {
                Ok(()) => file["task_id"] = json!(id),
                Err(reason) => {
                    failed += 1;
                    file["task_sync_reason"] = json!(reason);
                }
            }
        }
    }
    scan["task_status"] = json!(if failed == 0 {
        "synced_partial"
    } else {
        "incomplete"
    });
}

/// 严格校验 0.2 原生首次证据形状与归属；历史字节变化只使指引过期，不改写旧记录。
/// 当前位置可复核时验证原生字符列；当前身份检查由导入/投影调用方独立执行。
pub(crate) fn valid_history_report(root: &Path, workspace: &str, report: &Value) -> bool {
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
        "native_evidence",
    ];
    if !exact(report, &keys) {
        return false;
    }
    let Some(path) = report["scope"].as_str().filter(|p| safe_path(p)) else {
        return false;
    };
    let language = if report["schema_version"] == "0.9.0" {
        "ruby"
    } else if report["schema_version"] == "0.6.0" {
        "zig"
    } else if report["schema_version"] == "0.5.0" {
        "swift"
    } else if report["schema_version"] == "0.4.0" {
        "kotlin"
    } else {
        "erlang"
    };
    if !Path::new(path).extension().is_some_and(|e| {
        if language == "ruby" {
            e == "rb"
        } else if language == "zig" {
            e == "zig"
        } else if language == "swift" {
            e == "swift"
        } else if language == "kotlin" {
            e == "kt"
        } else {
            e == "erl" || e == "hrl"
        }
    }) {
        return false;
    }
    let (checker, reason, fingerprint) =
        crate::syntax_confirmation::identity(workspace, path, language);
    if !matches!(
        report["schema_version"].as_str(),
        Some("0.2.0" | "0.4.0" | "0.5.0" | "0.6.0" | "0.9.0")
    ) || report["report_type"] != "syntax_confirmation_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || report["execution"] != "incomplete"
        || report["checker_id"] != checker
        || report["reason_code"] != reason
        || report["blocker_id"] != format!("CG-B-{}", &fingerprint[..32])
        || report["fingerprint"] != fingerprint
        || report["build_root"] != "."
        || report["language"] != language
        || report["affected_paths"] != json!([path])
        || report["observations"] != json!([])
        || !report["run_id"]
            .as_str()
            .and_then(|r| r.strip_prefix("syntax-confirm-"))
            .is_some_and(|r| {
                r.split_once('-')
                    .is_some_and(|(p, n)| p.parse::<u32>().is_ok() && n.parse::<u128>().is_ok())
            })
    {
        return false;
    }
    let evidence = &report["native_evidence"];
    if !exact(evidence, &["target", "tool_path", "native"])
        || !exact(&evidence["target"], &["path", "language", "source_sha256"])
        || evidence["target"]["path"] != path
        || evidence["target"]["language"] != language
        || !evidence["target"]["source_sha256"]
            .as_str()
            .is_some_and(valid_sha)
        || !(evidence["tool_path"]
            .as_str()
            .is_some_and(|p| Path::new(p).is_absolute())
            || (evidence["tool_path"].is_null() && evidence["native"]["tool_sha256"].is_null()))
    {
        return false;
    }
    let current = read_bounded_regular_file(&root.join(path), 1024 * 1024)
        .ok()
        .filter(|b| evidence["target"]["source_sha256"] == digest(b));
    if language == "ruby" {
        crate::ruby_syntax_probe::valid_observation(&evidence["native"], current.as_deref())
    } else if language == "zig" {
        crate::syntax_task_recheck::valid_zig_evidence(root, evidence)
    } else if language == "swift" {
        crate::swift_syntax_probe::valid_native_observation(&evidence["native"], current.as_deref())
    } else if language == "kotlin" {
        codeguard_adapters::valid_kotlin_native_observation(&evidence["native"], current.as_deref())
    } else {
        crate::erlang_syntax_probe::valid_native_observation(
            &evidence["native"],
            current.as_deref(),
        )
    }
}

/// 读取已消费的最新原生扫描证据；返回内部指引投影和真实报告引用，不冒充 task verify。
pub(crate) fn latest(
    root: &Path,
    brief: &Value,
) -> Result<Option<(u128, Value, String)>, &'static str> {
    if !matches!(
        crate::syntax_task_recheck::original(root, brief)?["language"].as_str(),
        Some("erlang" | "kotlin" | "swift" | "zig" | "ruby")
    ) {
        return Ok(None);
    }
    let Some(id) = brief["task_id"].as_str() else {
        return Ok(None);
    };
    let workspace = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
        .ok_or("workspace_invalid")?;
    let mut latest = None;
    let entries = std::fs::read_dir(root.join(".codeguard/reports"))
        .map_err(|_| "native_history_unavailable")?;
    for (index, entry) in entries.enumerate() {
        if index >= 1000 {
            return Err("native_history_budget_exceeded");
        }
        let entry = entry.map_err(|_| "native_history_unavailable")?;
        let path = entry.path();
        let Some(run) = path
            .file_stem()
            .and_then(|p| p.to_str())
            .filter(|r| r.starts_with("syntax-confirm-"))
        else {
            continue;
        };
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let bytes = read_bounded_regular_file(&path, 1024 * 1024)
            .map_err(|_| "native_history_unavailable")?;
        let report =
            codeguard_adapters::parse_unique_json(&bytes).map_err(|_| "native_history_invalid")?;
        if !matches!(
            report["schema_version"].as_str(),
            Some("0.2.0" | "0.4.0" | "0.5.0" | "0.6.0" | "0.9.0")
        ) || report["blocker_id"] != id
        {
            continue;
        }
        let sequence = run
            .rsplit('-')
            .next()
            .and_then(|n| n.parse::<u128>().ok())
            .ok_or("native_history_invalid")?;
        if latest.as_ref().is_some_and(|(n, _, _)| *n >= sequence) {
            continue;
        }
        let marker = read_bounded_regular_file(
            &root.join(format!(".codeguard/state/consumed/{run}.json")),
            4096,
        )
        .ok()
        .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok());
        let Some(marker) = marker else {
            continue;
        };
        if !valid_history_report(root, &workspace, &report)
            || report["run_id"] != run
            || marker["schema_version"] != "0.1.0"
            || marker["workspace_id"] != workspace
            || marker["run_id"] != run
            || marker["report_sha256"] != digest(&bytes)
        {
            return Err("native_history_identity_conflict");
        }
        let mut view = report["native_evidence"].clone();
        view["run_id"] = json!(run);
        view["input_stable"] = json!(crate::syntax_task_recheck::inputs_current(root, &view));
        latest = Some((sequence, view, digest(&bytes)));
    }
    Ok(latest)
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && Path::new(path).extension().is_some_and(|e| {
            e == "erl" || e == "hrl" || e == "kt" || e == "swift" || e == "zig" || e == "rb"
        })
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
