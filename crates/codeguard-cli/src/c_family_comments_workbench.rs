//! 原生文档观察绑定已有工作台，不初始化或签发关闭。
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// 定位已有工作台；显式范围越界在原生启动前拒绝。
pub(crate) fn resolve_root(
    source: &Path,
    explicit: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    // 先拒绝链接/父跳转等不适用源码；不能按链接目标去写另一项目的工作台。
    crate::plain_syntax_source::read_plain_source(source)
        .map_err(|_| "C/C++工作台源码范围不适用")?;
    let absolute = source.canonicalize().map_err(|_| "C/C++工作台源码不可读")?;
    if let Some(root) = explicit {
        let root = root.canonicalize().map_err(|_| "C/C++工作台根不可读")?;
        if !root.is_dir() || !absolute.starts_with(&root) {
            return Err("C/C++文档源码不属于显式工作台".into());
        }
        return Ok(Some(root));
    }
    Ok(absolute
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .find(|root| std::fs::symlink_metadata(root.join(".codeguard")).is_ok())
        .map(Path::to_path_buf))
}
/// 保存并同步局部观察；中断后不开始新的持久化阶段。
pub(crate) fn connect(root: &Path, tool: &Path, report: &mut Value, deadline: Instant) {
    report["schema_version"] = json!("0.4.0");
    report["task_workflow_status"] = json!("partial");
    let argv = report["verification_command"]
        .as_array_mut()
        .expect("固定复检命令为数组");
    argv.pop();
    argv.push(json!("--workspace"));
    argv.push(json!(root));
    argv.push(json!("--format=json"));
    report["workbench"] = json!({"status":"incomplete","reason":"clang_documentation_workspace_unavailable","workspace_id":null,"run_id":null,"task_ids":[],"sync":null});
    if codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline {
        report["workbench"]["reason"] = json!("request_interrupted");
        return;
    }
    let Some(id) = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
    else {
        return;
    };
    let Some(path) = report["path"]
        .as_str()
        .and_then(|s| Path::new(s).strip_prefix(root).ok())
        .and_then(Path::to_str)
    else {
        return;
    };
    let run = format!(
        "clangdoc-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let packet = json!({"schema_version":"0.1.0","report_type":"clang_documentation_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":run,"path":path,"language":report["language"],"standard":report["documentation_configuration"]["standard"],"profile":"clang-documentation-v1","selected_tool":tool,"source_sha256":report["source_sha256"],"local_scan_complete":report["local_scan_complete"],"native":report["native"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    report["workbench"]["workspace_id"] = json!(id);
    report["workbench"]["run_id"] = json!(run);
    if !crate::work_sync::c_family_comments_report::valid_shape(&packet) {
        report["workbench"]["reason"] = json!("clang_documentation_packet_invalid");
        return;
    }
    if crate::work_sync::save_local_report(root, &packet).is_err() {
        report["workbench"]["reason"] = json!("clang_documentation_report_write_failed");
        return;
    }
    if codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline {
        report["workbench"]["reason"] = json!("request_interrupted");
        return;
    }
    match crate::work_sync::sync_local_workspace(root) {
        Ok(s) if s.failed_reports == 0 => {
            report["workspace_binding"] = json!("bound");
            report["workbench"]["status"] = json!("synced_partial");
            report["workbench"]["reason"] = Value::Null;
            report["workbench"]["sync"] = json!({"new_findings":s.new_findings,"new_blockers":s.new_blockers,"imported_reports":s.imported_reports,"already_consumed_reports":s.already_consumed_reports,"historical_findings":s.historical_findings});
            report["workbench"]["task_ids"] = json!(
                crate::work_sync::c_family_comments_report::task_ids(root, &packet)
            );
            if !codeguard_runtime::sigint_cancellation_requested() && Instant::now() < deadline {
                match crate::next_command::read_local_brief_for_checker(
                    root,
                    crate::work_sync::c_family_comments_report::checker(&packet),
                ) {
                    Ok(next) => {
                        report["next"] = if next["repair_brief"].is_null() {
                            Value::Null
                        } else {
                            next
                        }
                    }
                    Err(_) => {
                        report["workbench"]["reason"] =
                            json!("clang_documentation_next_unavailable")
                    }
                }
            }
        }
        _ => report["workbench"]["reason"] = json!("clang_documentation_sync_failed"),
    }
}
