//! 从已绑定本地事实恢复缺失的任务 Markdown；不授权工具执行或任务关闭。

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// 在调用方工作区锁内恢复缺失投影，返回恢复数量；原有普通文件保持原字节。
pub(super) fn recover_missing(root: &Path, consumed_only: bool) -> Result<u64, &'static str> {
    let tasks = root.join(".codeguard/tasks");
    let findings = root.join(".codeguard/findings");
    let state = root.join(".codeguard/state");
    let mut entries = fs::read_dir(&findings)
        .map_err(|_| "task_projection_facts_unavailable")?
        .take(1001)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "task_projection_facts_unavailable")?;
    if entries.len() > 1000 {
        return Err("task_projection_limit_exceeded");
    }
    entries.sort_by_key(|entry| entry.file_name());
    let mut planned = Vec::new();
    for entry in entries {
        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| "task_projection_id_invalid")?;
        if !crate::next_command::safe_id(&id) {
            // 非任务目录继续由 next/status 明示异常，不让独立投影恢复阻断有效报告导入。
            continue;
        }
        let path = tasks.join(format!("{id}.md"));
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => continue,
            Ok(_) => return Err("task_projection_path_conflict"),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => return Err("task_projection_path_unavailable"),
        }
        let fact_bytes = super::read_bounded_file(&entry.path().join("finding.json"), 128 * 1024)?;
        let fact = codeguard_adapters::parse_unique_json(&fact_bytes)
            .map_err(|_| "task_projection_fact_invalid")?;
        let run = fact["first_run_id"]
            .as_str()
            .filter(|run| super::safe_run_id(run))
            .ok_or("task_projection_origin_invalid")?;
        if consumed_only
            && !root
                .join(format!(".codeguard/state/consumed/{run}.json"))
                .exists()
        {
            // 尚未推进消费游标的中间态交给现有导入事务重放，不能冒充已提交事实。
            continue;
        }
        // 与 next 共用完整事实/历史校验；Markdown 缺失不取消已记录问题。
        let brief = crate::next_command::read_task_brief_for_projection(root, &id)?;
        validate_origin(root, &fact, &brief)?;
        let text = render(&brief)?;
        if text.len() > 128 * 1024 {
            return Err("task_projection_too_large");
        }
        planned.push((path, text));
    }
    // 全部缺失投影先完成验证，再创建新文件；原子不可覆盖写入拒绝编辑器竞态。
    for (path, text) in &planned {
        super::write_once(path, text.as_bytes(), &state)
            .map_err(|_| "task_projection_write_failed")?;
    }
    Ok(planned.len() as u64)
}

fn validate_origin(root: &Path, fact: &Value, brief: &Value) -> Result<(), &'static str> {
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .ok_or("task_projection_origin_invalid")?;
    let expected = brief["evidence_ref"]["first_report_sha256"]
        .as_str()
        .ok_or("task_projection_origin_invalid")?;
    let bytes = super::read_bounded_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        super::MAX_REPORT_BYTES,
    )?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err("task_projection_origin_changed");
    }
    let report = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "task_projection_origin_invalid")?;
    if report["workspace_id"] != fact["workspace_id"] || report["run_id"] != run {
        return Err("task_projection_origin_conflict");
    }
    let marker = super::read_bounded_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )?;
    let expected_marker = serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":fact["workspace_id"],"run_id":run,"report_sha256":expected})).map_err(|_| "task_projection_marker_invalid")?;
    if marker != expected_marker {
        return Err("task_projection_origin_not_consumed");
    }
    Ok(())
}

fn render(brief: &Value) -> Result<String, &'static str> {
    let id = brief["task_id"]
        .as_str()
        .ok_or("task_projection_id_invalid")?;
    let mut text = format!(
        "# CodeGuard 任务 {id}\n\n此文件由已绑定本地事实恢复，仅为可读投影。勾选、备注或删除不能关闭问题。请用 task show 查询当前指引。\n"
    );
    let sections = [
        (
            "问题证据",
            json!({"evidence_ref":brief["evidence_ref"],"native_confirmation_ref":brief["native_confirmation_ref"]}),
        ),
        (
            "规则依据",
            json!({"kind":brief["kind"],"checker_id":brief["checker_id"],"native_rule_id":brief["native_rule_id"],"reason_code":brief["reason_code"]}),
        ),
        (
            "允许修改的范围",
            json!({"scope":brief["scope"],"affected_paths":brief["affected_paths"],"constraints":brief["constraints"],"disposition":brief["disposition"]}),
        ),
        ("修复步骤", brief["step"].clone()),
        ("复检命令", redact_absolute_paths(&brief["recheck_argv"])),
        ("历史尝试", brief["history"].clone()),
        ("关闭条件", brief["closure_condition"].clone()),
    ];
    for (heading, value) in sections {
        let data =
            serde_json::to_string_pretty(&value).map_err(|_| "task_projection_encoding_failed")?;
        text.push_str(&format!("\n## {heading}\n\n```json\n{data}\n```\n"));
    }
    Ok(text)
}

fn redact_absolute_paths(value: &Value) -> Value {
    match value {
        Value::String(path) if Path::new(path).is_absolute() => json!("<需核验的绝对路径>"),
        Value::Array(values) => Value::Array(values.iter().map(redact_absolute_paths).collect()),
        _ => value.clone(),
    }
}
