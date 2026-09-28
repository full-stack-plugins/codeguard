//! 项目可写的运行预算默认值；不承载质量策略或例外授权。

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use codeguard_runtime::read_bounded_regular_file;
use serde::Deserialize;

use crate::check_budget::{parse_check_jobs, parse_check_timeout};

const MAX_RUNTIME_OPTIONS_BYTES: u64 = 4096;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// 项目内可选的版本化运行参数，仅允许指定检查预算。
struct ProjectRuntimeOptions {
    schema_version: String,
    document_type: String,
    timeout: String,
    jobs: Option<usize>,
}

/// 通过一次文件读取取得同一修订的检查预算。
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectRuntimeBudget {
    /// 检查总截止时间预算。
    pub timeout_ms: u64,
    /// 1.1 可选的并发任务上限。
    pub jobs: Option<usize>,
}

/// 读取版本化的项目超时默认值；缺文件表示沿用内置值，坏文件必须显式失败。
pub(crate) fn read_project_timeout(root: &Path) -> Result<Option<u64>, String> {
    read_project_runtime(root).map(|budget| budget.map(|budget| budget.timeout_ms))
}

/// 读取并严格校验同一版本化项目预算文件；质量策略字段一律拒绝。
pub(crate) fn read_project_runtime(root: &Path) -> Result<Option<ProjectRuntimeBudget>, String> {
    let directory = root.join(".codeguard");
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("项目运行配置目录不可读取".into()),
    };
    if !metadata.file_type().is_dir() {
        return Err("项目运行配置目录不是普通目录".into());
    }
    let path = directory.join("runtime.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("项目运行配置不可读取".into()),
    };
    if !metadata.file_type().is_file() {
        return Err("项目运行配置必须是普通文件".into());
    }
    let bytes = read_bounded_regular_file(&path, MAX_RUNTIME_OPTIONS_BYTES)
        .map_err(|_| "项目运行配置不可读取或超出上限")?;
    let options: ProjectRuntimeOptions =
        serde_json::from_slice(&bytes).map_err(|_| "项目运行配置格式或字段无效")?;
    if !matches!(options.schema_version.as_str(), "1.0" | "1.1")
        || options.document_type != "codeguard_runtime_options"
        || (options.schema_version == "1.0" && options.jobs.is_some())
    {
        return Err("项目运行配置协议版本或类型无效".into());
    }
    let timeout_ms =
        parse_check_timeout(&options.timeout).map_err(|_| "项目运行配置 timeout 无效")?;
    let jobs = options
        .jobs
        .map(|value| parse_check_jobs(&value.to_string()))
        .transpose()
        .map_err(|_| "项目运行配置 jobs 无效")?;
    Ok(Some(ProjectRuntimeBudget { timeout_ms, jobs }))
}
