//! 检查类命令共享的有界执行预算解析；调用方负责将同一截止时间传给所有子任务。

use serde_json::{Value, json};
use std::path::Path;

use crate::project_runtime_options::{read_project_runtime, read_project_timeout};

/// 检查类命令默认整轮预算，以毫秒表示。
pub const DEFAULT_CHECK_TIMEOUT_MS: u64 = 30 * 60 * 1000;
const MAX_TIMEOUT_MS: u64 = 24 * 60 * 60 * 1000;
const REGISTERED_TIMEOUT_ENV: &str = "CODEGUARD_TIMEOUT";
const REGISTERED_JOBS_ENV: &str = "CODEGUARD_JOBS";
const MAX_JOBS: usize = 64;

/// 超时预算及来源。
pub type TimeoutSelection = (u64, &'static str);
/// 并行预算及来源。
pub type JobsSelection = (usize, &'static str);

/// 解析检查类命令的正数持续时间；拒绝无单位、负数和超出协议上限的值。
pub fn parse_check_timeout(value: &str) -> Result<u64, String> {
    let (digits, unit) = if let Some(value) = value.strip_suffix("ms") {
        (value, 1_u64)
    } else if let Some(value) = value.strip_suffix('s') {
        (value, 1_000)
    } else if let Some(value) = value.strip_suffix('m') {
        (value, 60_000)
    } else if let Some(value) = value.strip_suffix('h') {
        (value, 3_600_000)
    } else {
        return Err("--timeout 需要 ms/s/m/h 单位".into());
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("--timeout 必须为正整数及 ms/s/m/h 单位".into());
    }
    digits
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(unit))
        .filter(|number| (1..=MAX_TIMEOUT_MS).contains(number))
        .ok_or("--timeout 必须在 1ms 至 24h 之间".into())
}

/// 记录公开反馈中实际选用的预算及来源；当前只对原生执行链提供硬截止时间。
pub fn budget_record(timeout_ms: u64, source: &str) -> Value {
    json!({
        "timeout_ms": timeout_ms,
        "source": source,
        "enforcement": "native_execution_only"
    })
}

/// 解析执行并发上限；零、非十进制或过大的值在启动任务前拒绝。
pub fn parse_check_jobs(value: &str) -> Result<usize, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("--jobs 必须是 1–64 的十进制整数".into());
    }
    value
        .parse::<usize>()
        .ok()
        .filter(|value| (1..=MAX_JOBS).contains(value))
        .ok_or("--jobs 必须在 1–64 之间".into())
}

/// 内置检查并发上限：最多四个，且不高于可用 CPU 并行度。
#[must_use]
pub fn default_check_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get().clamp(1, 4))
        .unwrap_or(1)
}

/// 选择 CLI、登记环境变量或内置并发上限；项目默认值在根路径确认后解析。
pub fn select_check_jobs(cli_jobs: Option<usize>) -> Result<(usize, &'static str), String> {
    if let Some(jobs) = cli_jobs {
        return Ok((jobs, "cli"));
    }
    if let Some(value) = std::env::var_os(REGISTERED_JOBS_ENV) {
        let value = value
            .into_string()
            .map_err(|_| format!("{REGISTERED_JOBS_ENV} 必须是 UTF-8 十进制整数"))?;
        let jobs = parse_check_jobs(&value)
            .map_err(|reason| format!("{REGISTERED_JOBS_ENV} 无效：{reason}"))?;
        return Ok((jobs, "registered_environment"));
    }
    Ok((default_check_jobs(), "builtin_default"))
}

/// 一次读取项目文件，分别选定超时与并发的项目默认值；高优先级值不被覆盖。
pub fn resolve_check_runtime(
    root: &Path,
    timeout: TimeoutSelection,
    jobs: JobsSelection,
) -> Result<(TimeoutSelection, JobsSelection), String> {
    if timeout.1 != "builtin_default" && jobs.1 != "builtin_default" {
        return Ok((timeout, jobs));
    }
    let Some(project) = read_project_runtime(root)? else {
        return Ok((timeout, jobs));
    };
    let timeout = if timeout.1 == "builtin_default" {
        (project.timeout_ms, "project_default")
    } else {
        timeout
    };
    let jobs = if jobs.1 == "builtin_default" {
        project
            .jobs
            .map_or(jobs, |value| (value, "project_default"))
    } else {
        jobs
    };
    Ok((timeout, jobs))
}

/// 在全项目反馈中区分配置并发上限、可执行节点与真实启动节点。
pub fn check_budget_record(
    timeout_ms: u64,
    timeout_source: &str,
    jobs_limit: usize,
    jobs_source: &str,
    native_task_count: usize,
    started_native_task_count: usize,
) -> Value {
    let mut record = budget_record(timeout_ms, timeout_source);
    let fields = record.as_object_mut().expect("预算记录是对象");
    fields.insert("jobs_limit".into(), json!(jobs_limit));
    fields.insert("jobs_source".into(), json!(jobs_source));
    fields.insert("native_task_count".into(), json!(native_task_count));
    fields.insert(
        "started_native_task_count".into(),
        json!(started_native_task_count),
    );
    record
}

/// 根据 CLI、已登记环境变量和内置默认值选择预算；CLI 显式值优先。
pub fn select_check_timeout(cli_timeout_ms: Option<u64>) -> Result<(u64, &'static str), String> {
    if let Some(timeout_ms) = cli_timeout_ms {
        return Ok((timeout_ms, "cli"));
    }
    if let Some(value) = std::env::var_os(REGISTERED_TIMEOUT_ENV) {
        let value = value
            .into_string()
            .map_err(|_| format!("{REGISTERED_TIMEOUT_ENV} 必须是 UTF-8 持续时间"))?;
        let timeout_ms = parse_check_timeout(&value)
            .map_err(|reason| format!("{REGISTERED_TIMEOUT_ENV} 无效：{reason}"))?;
        return Ok((timeout_ms, "registered_environment"));
    }
    Ok((DEFAULT_CHECK_TIMEOUT_MS, "builtin_default"))
}

/// 已完成 CLI/环境选择后，只有内置默认值才可被项目运行默认值替换。
pub fn resolve_project_default(
    root: &Path,
    timeout_ms: u64,
    source: &'static str,
) -> Result<(u64, &'static str), String> {
    if source != "builtin_default" {
        return Ok((timeout_ms, source));
    }
    match read_project_timeout(root)? {
        Some(project_timeout_ms) => Ok((project_timeout_ms, "project_default")),
        None => Ok((timeout_ms, source)),
    }
}
