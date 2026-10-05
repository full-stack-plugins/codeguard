//! 项目Shell的逐文件原生观察，共享预算并绑定显式工作区，不执行用户脚本。
use crate::{shellcheck_config::ShellCheckConfig, shellcheck_probe};
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
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn configuration(c: &Result<ShellCheckConfig, &'static str>) -> Value {
    match c {
        Ok(c) => c.report(),
        Err(reason) => {
            json!({"status":if *reason=="shellcheck_external_sources_unverified"{"unknown"}else{"invalid"},"source_path":null,"sha256":null,"reason":reason,"global_configuration":"not_loaded"})
        }
    }
}
/// 从声明或显式参数观察方言；缺少依据时不猜测默认bash。
fn dialect(path: &Path, bytes: &[u8], fallback: Option<&str>) -> String {
    let text = std::str::from_utf8(bytes).unwrap_or("");
    if let Some(rest) = text.lines().next().and_then(|l| l.strip_prefix("#!")) {
        let mut words = rest.split_whitespace();
        let first = words.next().unwrap_or("");
        let name = Path::new(first)
            .file_name()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let name = if name == "env" {
            words
                .find(|w| !matches!(*w, "-S" | "--") && !w.contains('='))
                .unwrap_or("")
        } else {
            name
        };
        if matches!(
            name,
            "sh" | "bash" | "dash" | "ksh" | "busybox" | "zsh" | "fish"
        ) {
            return name.into();
        }
        return "unknown".into();
    }
    match path.extension().and_then(|p| p.to_str()) {
        Some("bash") => "bash".into(),
        Some("zsh") => "zsh".into(),
        Some("fish") => "fish".into(),
        _ => fallback.unwrap_or("unknown").into(),
    }
}
/// 观察发现的相对文件，保留超限数量与未完整状态。
pub(crate) fn observe(
    root: &Path,
    paths: &BTreeSet<String>,
    tool: Option<PathBuf>,
    fallback: Option<&str>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let explicit = tool.is_some();
    let tool = tool.or_else(crate::shell_lint_command::discover_tool);
    let mut files = Vec::new();
    let mut native_task_started = false;
    for relative in paths.iter().take(MAX_FILES) {
        let path = root.join(relative);
        let source = if path.canonicalize().ok().as_deref() == Some(path.as_path()) {
            read_bounded_regular_file(&path, 1024 * 1024).ok()
        } else {
            None
        };
        let source = source.filter(|b| std::str::from_utf8(b).is_ok());
        let language = source
            .as_ref()
            .map(|b| dialect(&path, b, fallback))
            .unwrap_or_else(|| "unknown".into());
        let config = ShellCheckConfig::capture(&path, None);
        let native = if cancelled.load(Ordering::Relaxed)
            || codeguard_runtime::sigint_cancellation_requested()
        {
            shellcheck_probe::unavailable("request_cancelled")
        } else if Instant::now() >= deadline {
            shellcheck_probe::unavailable("request_deadline_exceeded")
        } else if let Some(source_bytes) = source.as_ref() {
            if language == "unknown" {
                shellcheck_probe::unavailable("shell_dialect_unresolved")
            } else if !matches!(
                language.as_str(),
                "sh" | "bash" | "dash" | "ksh" | "busybox"
            ) || crate::shell_lint_command::unsupported_file(&path)
                || !crate::shell_lint_command::supported_shebang(source_bytes)
            {
                shellcheck_probe::unavailable("shell_dialect_unsupported")
            } else {
                match (&config, tool.as_deref()) {
                    (Err(reason), _) => shellcheck_probe::unavailable(reason),
                    (Ok(c), Some(t)) => {
                        // 表示进入原生任务阶段，不把失败或取消解释为尚未开始。
                        native_task_started = true;
                        shellcheck_probe::observe(
                            t,
                            &path,
                            source_bytes,
                            &language,
                            c,
                            deadline,
                            cancelled,
                        )
                    }
                    _ => shellcheck_probe::unavailable("shellcheck_tool_not_found"),
                }
            }
        } else {
            shellcheck_probe::unavailable("shell_source_unavailable_or_invalid")
        };
        let stable = source.as_ref().is_some_and(|b| {
            read_bounded_regular_file(&path, 1024 * 1024).is_ok_and(|now| now == *b)
        }) && config.as_ref().is_ok_and(|c| c.current(&path));
        files.push(json!({"schema_version":"0.1.0","path":relative,"source_sha256":source.as_ref().map(|b|digest(b)),"dialect":language,"input_stable":stable,"native":native,"project_configuration":configuration(&config),"task_workflow_status":"not_integrated","workbench":null}));
    }
    let mut report = json!({"schema_version":"0.1.0","report_type":"shell_native_scan","scope":"bounded_discovered_shell_files","tool_selection":{"source":if explicit{"explicit"}else if tool.is_some(){"path"}else{"not_found"},"executable":tool,"resolved":tool.as_ref().and_then(|p|p.canonicalize().ok())},"source_file_count":paths.len(),"unobserved_count":paths.len().saturating_sub(MAX_FILES),"scope_stable":true,"local_check_complete":false,"native_task_started":native_task_started,"files":files,"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    refresh(root, &mut report, deadline);
    report
}
/// 复核范围、源码、配置和工具入口；失效诊断只保留为历史调查。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let tool = report["tool_selection"]["executable"]
        .as_str()
        .map(Path::new);
    let resolved = tool.and_then(|p| p.canonicalize().ok());
    let sha = resolved
        .as_ref()
        .and_then(|p| read_bounded_regular_file(p, 128 * 1024 * 1024).ok())
        .map(|b| digest(&b));
    let scope_stable = report["scope_stable"] == true;
    let entry_stable = report["tool_selection"]["resolved"] == json!(resolved);
    for f in report["files"].as_array_mut().into_iter().flatten() {
        let Some(relative) = f["path"].as_str() else {
            continue;
        };
        let path = root.join(relative);
        let stable = scope_stable
            && Instant::now() < deadline
            && path.canonicalize().ok().as_deref() == Some(path.as_path())
            && read_bounded_regular_file(&path, 1024 * 1024)
                .is_ok_and(|b| f["source_sha256"] == digest(&b))
            && ShellCheckConfig::capture(&path, None)
                .is_ok_and(|c| c.report() == f["project_configuration"])
            && entry_stable
            && f["native"]["tool_sha256"]
                .as_str()
                .is_none_or(|s| sha.as_deref() == Some(s));
        f["input_stable"] = json!(f["input_stable"] == true && stable);
    }
    report["local_check_complete"] = json!(
        scope_stable
            && report["unobserved_count"] == 0
            && report["files"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()
                    && rows.iter().all(|f| f["input_stable"] == true
                        && matches!(
                            f["native"]["status"].as_str(),
                            Some("completed" | "diagnostics_observed")
                        )))
    );
}
/// 在调用方范围复核后同步同一工作区任务，不自动初始化。
pub(crate) fn connect(root: &Path, report: &mut Value, deadline: Instant) {
    let mut connected = false;
    let mut failed = false;
    for f in report["files"].as_array_mut().into_iter().flatten() {
        let Some(path) = f["path"].as_str().map(|p| root.join(p)) else {
            continue;
        };
        if !f["source_sha256"].is_string() {
            failed = true;
            continue;
        }
        crate::shell_lint_workbench::connect_in_workspace(root, &path, None, f, deadline);
        connected |= f["workbench"].is_object();
        failed |= f["workbench"].is_object() && f["workbench"]["status"] != "synced_partial";
    }
    report["task_status"] = json!(if failed {
        "incomplete"
    } else if connected {
        "synced_partial"
    } else {
        "not_connected"
    });
}
