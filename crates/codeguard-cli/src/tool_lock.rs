//! 工具锁的严格只读协议；解析成功不代表签发来源可信或工具已经安装。

use codeguard_core::CANDIDATE_PLATFORMS;
use serde_json::Value;
use std::collections::BTreeSet;

/// 单个平台上固定的原生工具、适配器和规则来源身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedTool {
    /// 工具规范 ID。
    pub id: String,
    /// 原生工具版本。
    pub version: String,
    /// 原生可执行文件 SHA-256。
    pub binary_sha256: String,
    /// 锁适用的平台。
    pub platform: String,
    /// 适配器 ID。
    pub adapter_id: String,
    /// 适配器版本。
    pub adapter_version: String,
    /// 规则包或内置规则的来源 ID。
    pub rule_source_id: String,
    /// 规则来源类别。
    pub rule_source_kind: String,
    /// 规则来源内容 SHA-256。
    pub rule_source_sha256: String,
    /// 安装/解析来源类别。
    pub origin_kind: String,
    /// 安装/解析来源引用。
    pub origin_ref: String,
    /// 可选运行时身份，例如 Maven 所需 JDK。
    pub runtime: Option<LockedRuntime>,
    /// 可选的委托发行包目录内容身份；入口脚本自身摘要不足以约束它。
    pub bundle: Option<LockedBundle>,
}

/// 目录内全部普通文件与目录的确定性内容身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedBundle {
    /// system 来源使用绝对路径；项目/缓存来源使用各自根目录下相对路径。
    pub root: String,
    /// 确定性目录树摘要。
    pub tree_sha256: String,
}

/// 外部运行时的固定身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedRuntime {
    /// 运行时 ID，例如 jdk。
    pub id: String,
    /// 运行时版本。
    pub version: String,
    /// 运行时可执行文件 SHA-256。
    pub binary_sha256: String,
}

/// 已通过结构验证的工具锁；仍需独立验证批准来源。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolLock {
    /// 锁文档 ID。
    pub lock_id: String,
    /// 全部平台工具条目。
    pub tools: Vec<LockedTool>,
}

impl ToolLock {
    /// 查找唯一工具与平台条目；重复键在解析时已被拒绝。
    #[must_use]
    pub fn find(&self, id: &str, platform: &str) -> Option<&LockedTool> {
        self.tools
            .iter()
            .find(|tool| tool.id == id && tool.platform == platform)
    }
}

/// 校验工具锁协议版本、字段、枚举、摘要及唯一性；未知 major 必须拒绝。
pub fn parse_tool_lock_document(bytes: &[u8]) -> Result<ToolLock, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let root = object(&value, "工具锁")?;
    check_keys(
        root,
        &["schema_version", "lock_id", "tools", "extensions"],
        "工具锁",
    )?;
    let version = required_string(root, "schema_version", "工具锁")?;
    let (major, _) = version
        .split_once('.')
        .ok_or("工具锁 schema_version 格式错误")?;
    let minor = version.split_once('.').expect("已检查分隔符").1;
    if major != "1" || minor.is_empty() || minor.parse::<u32>().is_err() {
        return Err("不支持的工具锁协议版本".into());
    }
    validate_extensions(root.get("extensions"))?;
    let lock_id = required_string(root, "lock_id", "工具锁")?.to_owned();
    let rows = root
        .get("tools")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or("工具锁 tools 不能为空")?;
    let mut tools = Vec::with_capacity(rows.len());
    let mut identities = BTreeSet::new();
    for row in rows {
        let tool = parse_tool(row)?;
        if !identities.insert((tool.id.clone(), tool.platform.clone())) {
            return Err(format!("重复工具平台身份：{}@{}", tool.id, tool.platform));
        }
        tools.push(tool);
    }
    Ok(ToolLock { lock_id, tools })
}

fn parse_tool(value: &Value) -> Result<LockedTool, String> {
    let row = object(value, "工具条目")?;
    check_keys(
        row,
        &[
            "id",
            "version",
            "binary_sha256",
            "platform",
            "adapter",
            "rule_source",
            "origin",
            "runtime",
            "bundle",
            "extensions",
        ],
        "工具条目",
    )?;
    validate_extensions(row.get("extensions"))?;
    let platform = required_string(row, "platform", "工具条目")?;
    if !CANDIDATE_PLATFORMS.contains(&platform) {
        return Err(format!("未知平台：{platform}"));
    }
    let adapter = object(row.get("adapter").ok_or("缺 adapter")?, "adapter")?;
    check_keys(adapter, &["id", "version"], "adapter")?;
    let rules = object(
        row.get("rule_source").ok_or("缺 rule_source")?,
        "rule_source",
    )?;
    check_keys(rules, &["kind", "id", "sha256"], "rule_source")?;
    let rule_kind = required_string(rules, "kind", "rule_source")?;
    if !matches!(rule_kind, "native_builtin" | "external_rulepack") {
        return Err("非法 rule_source.kind".into());
    }
    let origin = object(row.get("origin").ok_or("缺 origin")?, "origin")?;
    check_keys(origin, &["kind", "ref"], "origin")?;
    let origin_kind = required_string(origin, "kind", "origin")?;
    if !matches!(origin_kind, "project_wrapper" | "managed_cache" | "system") {
        return Err("非法 origin.kind".into());
    }
    let runtime = row
        .get("runtime")
        .map(|value| {
            let runtime = object(value, "runtime")?;
            check_keys(runtime, &["id", "version", "binary_sha256"], "runtime")?;
            Ok::<LockedRuntime, String>(LockedRuntime {
                id: required_string(runtime, "id", "runtime")?.to_owned(),
                version: required_string(runtime, "version", "runtime")?.to_owned(),
                binary_sha256: digest(runtime, "binary_sha256", "runtime")?,
            })
        })
        .transpose()?;
    let bundle = row
        .get("bundle")
        .map(|value| {
            let bundle = object(value, "bundle")?;
            check_keys(bundle, &["root", "tree_sha256"], "bundle")?;
            Ok::<LockedBundle, String>(LockedBundle {
                root: required_string(bundle, "root", "bundle")?.to_owned(),
                tree_sha256: digest(bundle, "tree_sha256", "bundle")?,
            })
        })
        .transpose()?;
    Ok(LockedTool {
        id: required_string(row, "id", "工具条目")?.to_owned(),
        version: required_string(row, "version", "工具条目")?.to_owned(),
        binary_sha256: digest(row, "binary_sha256", "工具条目")?,
        platform: platform.to_owned(),
        adapter_id: required_string(adapter, "id", "adapter")?.to_owned(),
        adapter_version: required_string(adapter, "version", "adapter")?.to_owned(),
        rule_source_id: required_string(rules, "id", "rule_source")?.to_owned(),
        rule_source_kind: rule_kind.to_owned(),
        rule_source_sha256: digest(rules, "sha256", "rule_source")?,
        origin_kind: origin_kind.to_owned(),
        origin_ref: required_string(origin, "ref", "origin")?.to_owned(),
        runtime,
        bundle,
    })
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} 必须是对象"))
}

fn check_keys(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{label} 包含未知字段：{key}"));
        }
    }
    Ok(())
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{label} 缺少非空 {key}"))
}

fn digest(
    object: &serde_json::Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<String, String> {
    let value = required_string(object, key, label)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("{label}.{key} 必须是小写 SHA-256"));
    }
    Ok(value.to_owned())
}

fn validate_extensions(value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let extensions = object(value, "extensions")?;
    if extensions
        .keys()
        .any(|key| !key.starts_with("x-") || key.len() < 3)
    {
        return Err("extensions 只能使用 x- 前缀".into());
    }
    Ok(())
}
