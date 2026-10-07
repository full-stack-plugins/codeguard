//! 公开占位观察自动连接本地工作台；稳定记录不授予专用复检或关闭权限。
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 保存同次扫描的独立占位包并复用既有消费事务；参数为根、工具、公开反馈及共享截止。
pub(crate) fn connect(root: &Path, tool: &Path, report: &mut Value, deadline: Instant) {
    report["placeholder_task_workflow_status"] = json!("partial");
    report["placeholder_workbench"] = json!({"status":"incomplete","reason":"clang_placeholder_workspace_unavailable","workspace_id":null,"run_id":null,"task_ids":[]});
    if interrupted(deadline) {
        report["placeholder_workbench"]["reason"] = json!("request_interrupted");
        return;
    }
    if report["workspace_binding"] != "bound" {
        return;
    }
    let Some(workspace) = report["workbench"]["workspace_id"].as_str() else {
        return;
    };
    let Some(run) = report["workbench"]["run_id"]
        .as_str()
        .map(|run| run.replacen("clangdoc-", "clangdocplaceholder-", 1))
    else {
        return;
    };
    let Some(path) = report["path"]
        .as_str()
        .and_then(|path| Path::new(path).strip_prefix(root).ok())
        .and_then(Path::to_str)
    else {
        return;
    };
    let packet = json!({"schema_version":"0.1.0","report_type":"clang_documentation_placeholder_workbench_observation","workspace_binding":"bound","workspace_id":workspace,"run_id":run,"path":path,"language":report["language"],"standard":report["documentation_configuration"]["standard"],"profile":"clang-documentation-placeholder-v1","selected_tool":tool,"source_sha256":report["source_sha256"],"local_scan_complete":report["local_scan_complete"],"native":report["native"],"structure":report["documentation_structure"],"placeholders":report["documentation_placeholders"]["observation"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    report["placeholder_workbench"]["workspace_id"] = packet["workspace_id"].clone();
    report["placeholder_workbench"]["run_id"] = packet["run_id"].clone();
    if !crate::work_sync::c_family_placeholder_report::valid_shape(&packet) {
        report["placeholder_workbench"]["reason"] =
            json!("clang_placeholder_observation_unavailable");
        return;
    }
    if crate::work_sync::save_local_report(root, &packet).is_err() {
        report["placeholder_workbench"]["reason"] = json!("clang_placeholder_report_write_failed");
        return;
    }
    if interrupted(deadline) {
        report["placeholder_workbench"]["reason"] = json!("request_interrupted");
        return;
    }
    match crate::work_sync::sync_local_workspace(root) {
        Ok(summary) if summary.failed_reports == 0 => {
            report["placeholder_workbench"]["status"] = json!("synced_partial");
            report["placeholder_workbench"]["reason"] = Value::Null;
            report["placeholder_workbench"]["task_ids"] = json!(
                crate::work_sync::c_family_placeholder_report::task_ids(root, &packet)
            );
        }
        _ => report["placeholder_workbench"]["reason"] = json!("clang_placeholder_sync_failed"),
    }
}
fn interrupted(deadline: Instant) -> bool {
    codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline
}
