//! 版本化检查请求的严格只读解析；解析成功不代表项目发现或执行完成。

use serde_json::{Map, Value};

/// 已验证的检查请求选择与操作参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckRequest {
    /// 调用方提供的非空请求 ID。
    pub request_id: String,
    /// 六类检查命令之一。
    pub command: String,
    /// 规范语言 ID 或 all；语言注册表校验属于命令解析层。
    pub language: String,
    /// 项目路径原文；后续观察层解析并验证真实身份。
    pub root: String,
    /// 内容来源类别。
    pub content_kind: String,
    /// ref/CI 等明确内容身份。
    pub content_identity: Option<String>,
    /// 输出格式。
    pub format: String,
    /// 是否要求受验证的离线执行。
    pub offline: bool,
    /// 总请求预算，毫秒。
    pub timeout_ms: u64,
    /// 最大并发数。
    pub jobs: u64,
}

/// 解析并拒绝未知协议 major、非法枚举与未声明的权威字段。
pub fn parse_check_request(bytes: &[u8]) -> Result<CheckRequest, String> {
    let value = codeguard_adapters::parse_unique_json(bytes).map_err(str::to_owned)?;
    let root = object(&value, "检查请求")?;
    keys(
        root,
        &[
            "schema_version",
            "report_type",
            "request_id",
            "command",
            "selection",
            "root",
            "content_source",
            "options",
            "extensions",
        ],
        "检查请求",
    )?;
    let version = string(root, "schema_version", "检查请求")?;
    let Some((major, minor)) = version.split_once('.') else {
        return Err("schema_version 格式错误".into());
    };
    if major != "1" || minor.is_empty() || minor.parse::<u32>().is_err() {
        return Err("未知检查请求协议版本".into());
    }
    if string(root, "report_type", "检查请求")? != "check_request" {
        return Err("检查请求 report_type 错误".into());
    }
    if let Some(extensions) = root.get("extensions") {
        let extensions = object(extensions, "extensions")?;
        if extensions
            .keys()
            .any(|key| !key.starts_with("x-") || key.len() < 3)
        {
            return Err("extensions 只能使用 x- 前缀".into());
        }
    }
    let command = string(root, "command", "检查请求")?;
    if !matches!(
        command,
        "lint" | "comments" | "dependencies" | "cve" | "security" | "build" | "check"
    ) {
        return Err("未知检查命令".into());
    }
    let selection = object(root.get("selection").ok_or("缺 selection")?, "selection")?;
    keys(selection, &["language"], "selection")?;
    let content = object(
        root.get("content_source").ok_or("缺 content_source")?,
        "content_source",
    )?;
    keys(content, &["kind", "identity"], "content_source")?;
    let content_kind = string(content, "kind", "content_source")?;
    if !matches!(
        content_kind,
        "working_tree" | "git_index" | "git_ref" | "ci"
    ) {
        return Err("未知内容来源".into());
    }
    let content_identity = content
        .get("identity")
        .map(|_| string(content, "identity", "content_source").map(str::to_owned))
        .transpose()?;
    if matches!(content_kind, "git_ref" | "ci") && content_identity.is_none() {
        return Err("不可变内容来源缺 identity".into());
    }
    let options = object(root.get("options").ok_or("缺 options")?, "options")?;
    keys(
        options,
        &["format", "offline", "timeout_ms", "jobs"],
        "options",
    )?;
    let format = string(options, "format", "options")?;
    if !matches!(format, "human" | "json" | "sarif") {
        return Err("未知输出格式".into());
    }
    let offline = options
        .get("offline")
        .and_then(Value::as_bool)
        .ok_or("offline 必须是布尔值")?;
    let timeout_ms = bounded_number(options, "timeout_ms", 1, 86_400_000)?;
    let jobs = bounded_number(options, "jobs", 1, 64)?;
    Ok(CheckRequest {
        request_id: string(root, "request_id", "检查请求")?.to_owned(),
        command: command.to_owned(),
        language: string(selection, "language", "selection")?.to_owned(),
        root: string(root, "root", "检查请求")?.to_owned(),
        content_kind: content_kind.to_owned(),
        content_identity,
        format: format.to_owned(),
        offline,
        timeout_ms,
        jobs,
    })
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} 必须是对象"))
}

fn keys(object: &Map<String, Value>, allowed: &[&str], label: &str) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{label} 包含未知字段：{key}"));
        }
    }
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, key: &str, label: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{label} 缺少非空 {key}"))
}

fn bounded_number(
    object: &Map<String, Value>,
    key: &str,
    min: u64,
    max: u64,
) -> Result<u64, String> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| format!("{key} 超出允许范围"))
}
