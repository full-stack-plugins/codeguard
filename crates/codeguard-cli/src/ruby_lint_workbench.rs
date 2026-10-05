//! Ruby 单文件候选与原生观察复用同一修复工作台；不重复执行解析器。
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 同步已经冻结并检查过的 Ruby 文件；参数为路径、反馈和截止时间，不初始化工作区。
pub(crate) fn connect(source: &Path, report: &mut Value, deadline: Instant) {
    crate::native_syntax_confirmation::connect_file(source, report, deadline);
    let Some(candidate) = report["syntax_precheck"].as_object() else {
        return;
    };
    if candidate
        .get("grammar_sha256")
        .and_then(Value::as_str)
        .is_none()
    {
        return;
    }
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
    let Some(path) = absolute.strip_prefix(root).ok().and_then(Path::to_str) else {
        return;
    };
    if report["workspace_binding"] != "bound" {
        return;
    }
    let mut recoveries = report["syntax_precheck"]["recoveries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    // worker 分组身份只用于本轮传输，历史候选协议保存明确的八项原始位置。
    for row in &mut recoveries {
        if let Some(object) = row.as_object_mut() {
            object.remove("group_id");
        }
    }
    let Some(asset) = codeguard_adapters::bundled_grammar_candidates()
        .ok()
        .and_then(|m| m.assets.into_iter().find(|a| a.language == "ruby"))
    else {
        return;
    };
    let incomplete = report["syntax_precheck"]["precheck"]["truncated_files"]
        .as_u64()
        .unwrap_or(0)
        > 0;
    let row = json!({"path":path,"language":"ruby","scope":"whole_file","byte_offset":0,
        "status":"candidate_observed","reason":if incomplete {json!("syntax_recovery_incomplete")} else {Value::Null},"grammar_qualified":false,
        "source_sha256":report["source_sha256"],"grammar_sha256":report["syntax_precheck"]["grammar_sha256"],
        "recovery_count":recoveries.len(),"recoveries":recoveries.iter().take(8).collect::<Vec<_>>(),"known_limitations":asset.known_limitations});
    let tasks = crate::syntax_confirmation::persist(root, &json!({"observations":[row]}), deadline);
    if let Some(id) = tasks["tasks"][0]["task_id"].as_str() {
        report["task_id"] = json!(id);
    }
    if tasks["status"] == "incomplete" {
        report["task_status"] = json!("incomplete");
        report["task_sync_reason"] = tasks["failures"][0]["reason"].clone();
    }
}
