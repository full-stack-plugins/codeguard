//! 单文件语法候选的共享历史恢复及有界终端反馈。
use serde_json::Value;

// 仅从已初始化工作区及经过完整工作台校验的现存任务恢复引用，不制造新任务或关闭证据。
pub(crate) fn pending_confirmation(
    root: &std::path::Path,
    relative: &str,
    language: &str,
) -> Result<Option<String>, &'static str> {
    let baseline = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "workspace_invalid")?
        .ok_or("workspace_not_initialized")?;
    let (checker, _, fingerprint) = crate::syntax_confirmation::identity(
        baseline.workspace_id().ok_or("workspace_id_unavailable")?,
        relative,
        language,
    );
    let id = format!("CG-B-{}", &fingerprint[..32]);
    // 检查父目录本身，避免通过可替换链接读取其他工作区的历史。
    for path in [
        root.join(".codeguard"),
        root.join(".codeguard/findings"),
        root.join(".codeguard/tasks"),
    ] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_dir()
                    && path.canonicalize().ok().as_deref() == Some(path.as_path()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("workspace_records_unavailable"),
        }
    }
    let directory = root.join(".codeguard/findings").join(&id);
    let projection = root.join(".codeguard/tasks").join(format!("{id}.md"));
    let absent = |path: &std::path::Path| -> Result<bool, &'static str> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(_) => Err("task_record_unavailable"),
        }
    };
    // 只有事实目录和投影均不存在才表示没有同范围任务；半缺失要求恢复记录。
    if absent(&directory)? && absent(&projection)? {
        return Ok(None);
    }
    let brief = crate::next_command::read_task_brief(root, &id)?;
    if brief["checker_id"] != checker || brief["kind"] != "blocker" || brief["scope"] != relative {
        return Err("task_identity_conflict");
    }
    Ok(Some(id))
}

/// 输出有界脱敏语法与结构位置；参数为本轮候选报告，不输出源码片段或标识符。
pub(crate) fn print_feedback(syntax: &Value) {
    println!(
        "内置语法候选初检：{}；原生 lint 尚未完成。",
        syntax["status"]
    );
    for row in syntax["observations"]
        .as_array()
        .into_iter()
        .flatten()
        .take(4)
    {
        println!(
            "疑似恢复节点 {}，结构候选 {}；检查原因 {}。",
            row["recovery_count"],
            row["structural_observation_count"].as_u64().unwrap_or(0),
            row["reason"]
        );
        for recovery in row["recoveries"].as_array().into_iter().flatten().take(8) {
            println!(
                "疑似语法 {}，零起始行 {}、UTF-8 字节列 {}；须原生确认。",
                recovery["kind"], recovery["start_row"], recovery["start_column_byte"]
            );
        }
        for structure in row["structural_observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "疑似结构规则 {}，零起始行 {}、UTF-8 字节列 {}；须原生确认。",
                structure["rule_id"], structure["start_row"], structure["start_column_byte"]
            );
        }
    }
}
