//! Shell原生观察接入已有工作台；不初始化、不签发关闭或改变原生结果。
use crate::{
    work_sync::{save_local_report, sync_local_workspace},
    workspace_refresh::read_workspace_baseline,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// 保存当前单文件事实并同步；参数为源码、显式rc、公开反馈及共同截止时间。
pub(crate) fn connect(
    source: &Path,
    requested: Option<&Path>,
    report: &mut Value,
    deadline: Instant,
) {
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
    connect_in_workspace(root, &absolute, requested, report, deadline);
}
/// 在项目检查指定根内保存观察，子工作台不改变任务归属；未初始化不创建目录。
pub(crate) fn connect_in_workspace(
    root: &Path,
    source: &Path,
    requested: Option<&Path>,
    report: &mut Value,
    deadline: Instant,
) {
    if std::fs::symlink_metadata(root.join(".codeguard")).is_err() {
        return;
    }
    let Ok(absolute) = source.canonicalize() else {
        return;
    };
    report["schema_version"] = json!("0.2.0");
    report["task_workflow_status"] = json!("partial");
    report["workbench"] = json!({"status":"incomplete","reason":"shell_workspace_unavailable","workspace_id":null,"run_id":null,"task_ids":[],"sync":null});
    if report["native"]["reason"] == "request_cancelled" {
        report["workbench"]["reason"] = json!("request_cancelled");
        return;
    }
    let Some(workspace_id) = read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
    else {
        return;
    };
    let Some(path) = absolute
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .filter(|s| crate::work_sync::shell_report::safe_path(s))
    else {
        return;
    };
    let run = format!(
        "shellcheck-{}-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let packet = json!({"schema_version":"0.1.0","report_type":"shellcheck_workbench_observation","workspace_binding":"bound","workspace_id":workspace_id,"run_id":run,"path":path,"source_sha256":report["source_sha256"],"dialect":report["dialect"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","input_stable":report["input_stable"],"native":report["native"],"project_configuration":report["project_configuration"],"requested_config":requested});
    report["workbench"]["workspace_id"] = json!(workspace_id);
    report["workbench"]["run_id"] = json!(run);
    if Instant::now() >= deadline {
        report["workbench"]["reason"] = json!("request_deadline_exceeded");
        return;
    }
    if save_local_report(root, &packet).is_err() {
        report["workbench"]["reason"] = json!("shell_report_write_failed");
        return;
    }
    match sync_local_workspace(root) {
        Ok(summary) if summary.failed_reports == 0 => {
            report["workbench"]["status"] = json!("synced_partial");
            report["workbench"]["reason"] = Value::Null;
            report["workbench"]["sync"] = json!({"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"imported_reports":summary.imported_reports,"already_consumed_reports":summary.already_consumed_reports,"historical_findings":summary.historical_findings});
            report["workbench"]["task_ids"] =
                json!(crate::work_sync::shell_report::task_ids(root, &packet));
        }
        _ => {
            report["workbench"]["reason"] = json!("shell_backlog_sync_failed");
        }
    }
}
