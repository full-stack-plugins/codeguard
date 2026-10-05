//! 独立lint与聚合check共用的Clippy持久化及修复指引投影。
use crate::work_sync::{save_local_report, sync_local_workspace};
use serde_json::{Value, json};
use std::path::Path;
/// 保存原生局部报告并幂等同步工作台；失败保留原生发现，零诊断不关闭任务。
pub(crate) fn persist_and_sync(root: &Path, report: &mut Value) {
    let (status, summary) = match save_local_report(root, report) {
        Ok(()) if report["workspace_binding"] == "bound" => match sync_local_workspace(root) {
            Ok(summary) if summary.failed_reports == 0 => (
                "synced_partial",
                json!({"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"imported_reports":summary.imported_reports,"historical_findings":summary.historical_findings,"failed_reports":summary.failed_reports}),
            ),
            _ => ("backlog_update_failed", Value::Null),
        },
        Ok(()) => ("not_initialized", Value::Null),
        Err(_) => ("backlog_update_failed", Value::Null),
    };
    report["backlog_status"] = json!(status);
    report["backlog_sync"] = summary;
    report["next"] = if status == "synced_partial" {
        crate::next_command::read_local_brief(root).unwrap_or(Value::Null)
    } else {
        Value::Null
    };
}
