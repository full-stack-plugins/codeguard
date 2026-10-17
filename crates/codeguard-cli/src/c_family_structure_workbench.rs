//! 原生AST结构局部反馈连接已有工作台，独立于原警告事实与可信关闭。
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 保存同次扫描的结构观察并归并文件策略任务；参数为已解析根/原工具/反馈和共享截止时间。
pub(crate) fn connect(root: &Path, tool: &Path, report: &mut Value, deadline: Instant) {
    report["structural_task_workflow_status"] = json!("partial");
    report["structural_next"] = Value::Null;
    report["structural_workbench"] = json!({"status":"incomplete","reason":"clang_structure_workspace_unavailable","workspace_id":null,"run_id":null,"task_ids":[]});
    if codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline {
        report["structural_workbench"]["reason"] = json!("request_interrupted");
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
        .map(|s| s.replacen("clangdoc-", "clangdocstruct-", 1))
    else {
        return;
    };
    let Some(path) = report["path"]
        .as_str()
        .and_then(|p| Path::new(p).strip_prefix(root).ok())
        .and_then(Path::to_str)
    else {
        return;
    };
    let packet = json!({"schema_version":"0.1.0","report_type":"clang_documentation_structure_workbench_observation","workspace_binding":"bound","workspace_id":workspace,"run_id":run,"path":path,"language":report["language"],"standard":report["documentation_configuration"]["standard"],"profile":"clang-documentation-structure-v1","selected_tool":tool,"source_sha256":report["source_sha256"],"local_scan_complete":report["local_scan_complete"],"native":report["native"],"structure":report["documentation_structure"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    report["structural_workbench"]["workspace_id"] = packet["workspace_id"].clone();
    report["structural_workbench"]["run_id"] = packet["run_id"].clone();
    if !crate::work_sync::c_family_structure_report::valid_shape(&packet) {
        report["structural_workbench"]["reason"] = json!("clang_structure_packet_invalid");
        return;
    }
    if crate::work_sync::save_local_report(root, &packet).is_err() {
        report["structural_workbench"]["reason"] = json!("clang_structure_report_write_failed");
        return;
    }
    if codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline {
        report["structural_workbench"]["reason"] = json!("request_interrupted");
        return;
    }
    match crate::work_sync::sync_local_workspace(root) {
        Ok(s) if s.failed_reports == 0 => {
            report["structural_workbench"]["status"] = json!("synced_partial");
            report["structural_workbench"]["reason"] = Value::Null;
            report["structural_workbench"]["task_ids"] = json!(
                crate::work_sync::c_family_structure_report::task_ids(root, &packet)
            );
            if let Ok(next) = crate::next_command::read_local_brief_for_checker(
                root,
                crate::work_sync::c_family_structure_report::checker(&packet),
            ) {
                if !next["repair_brief"].is_null() {
                    report["structural_next"] = next.clone();
                    if report["next"].is_null() {
                        report["next"] = next;
                    }
                }
            } else {
                report["structural_workbench"]["reason"] =
                    json!("clang_structure_next_unavailable");
            }
        }
        _ => report["structural_workbench"]["reason"] = json!("clang_structure_sync_failed"),
    }
}
