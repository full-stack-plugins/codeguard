//! 检查计划协议的只读验证；计划完整返回不表示任何质量义务已经执行。

use codeguard_core::{CHECK_CATEGORIES, check_kind_belongs_to_category};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// 经结构和图校验的计划，保留完整原始文档供后续领域服务使用。
#[derive(Clone, Debug)]
pub struct CheckPlan {
    /// 请求身份。
    pub request_id: String,
    /// 计划身份。
    pub plan_id: String,
    /// 唯一义务 ID。
    pub obligation_ids: Vec<String>,
    /// 唯一执行任务 ID。
    pub task_ids: Vec<String>,
    document: Value,
}

impl CheckPlan {
    /// 借用完整且已验证的计划文档；调用方不得据此声称执行完成。
    #[must_use]
    pub fn document(&self) -> &Value {
        &self.document
    }
}

/// 拒绝未知协议、坏身份、丢失义务、悬挂依赖和任务环。
pub fn parse_check_plan(bytes: &[u8]) -> Result<CheckPlan, String> {
    let document: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let root = object(&document, "计划")?;
    keys(
        root,
        &[
            "schema_version",
            "report_type",
            "request_id",
            "plan_id",
            "content_identity",
            "policy_identity",
            "discovery_complete",
            "obligations",
            "tasks",
            "edges",
            "unresolved_conditions",
            "extensions",
        ],
        "计划",
    )?;
    version(root)?;
    if string(root, "report_type", "计划")? != "check_plan" {
        return Err("计划 report_type 错误".into());
    }
    extensions(root.get("extensions"))?;
    let request_id = string(root, "request_id", "计划")?.to_owned();
    let plan_id = string(root, "plan_id", "计划")?.to_owned();
    validate_identities(root)?;
    root.get("discovery_complete")
        .and_then(Value::as_bool)
        .ok_or("discovery_complete 必须是布尔值")?;

    let mut obligations = BTreeMap::new();
    for item in array(root, "obligations", "计划")? {
        let item = object(item, "义务")?;
        keys(
            item,
            &[
                "id",
                "module",
                "language",
                "category",
                "check_kind",
                "applicability",
                "required",
                "expected_targets",
                "rule_ids",
                "reason",
            ],
            "义务",
        )?;
        let id = string(item, "id", "义务")?.to_owned();
        string(item, "module", "义务")?;
        string(item, "language", "义务")?;
        let category = string(item, "category", "义务")?;
        if !CHECK_CATEGORIES.contains(&category) {
            return Err(format!("{id}: 未知类别"));
        }
        if let Some(check_kind) = item.get("check_kind") {
            let check_kind = check_kind
                .as_str()
                .ok_or_else(|| format!("{id}: 检测族必须是字符串"))?;
            if !check_kind_belongs_to_category(check_kind, category) {
                return Err(format!("{id}: 检测族与类别不匹配或未知"));
            }
        } else if root["schema_version"] != "1.0" {
            return Err(format!("{id}: 缺检测族"));
        }
        let applicability = string(item, "applicability", "义务")?;
        if !matches!(applicability, "applicable" | "not_applicable" | "unknown") {
            return Err(format!("{id}: 未知适用性"));
        }
        let required = item
            .get("required")
            .and_then(Value::as_bool)
            .ok_or("义务 required 必须是布尔值")?;
        let targets = unique_strings(item, "expected_targets", "义务")?;
        unique_strings(item, "rule_ids", "义务")?;
        if applicability == "applicable" && targets.is_empty() {
            return Err(format!("{id}: 适用义务缺目标"));
        }
        if applicability == "not_applicable"
            && (!targets.is_empty() || string(item, "reason", "义务").is_err())
        {
            return Err(format!("{id}: 不适用义务缺结构性依据"));
        }
        if obligations
            .insert(id.clone(), (required, applicability.to_owned()))
            .is_some()
        {
            return Err(format!("重复义务 ID：{id}"));
        }
    }

    let mut tasks = BTreeMap::new();
    let mut covered = BTreeSet::new();
    for item in array(root, "tasks", "计划")? {
        let item = object(item, "执行任务")?;
        keys(
            item,
            &[
                "id",
                "state",
                "adapter_id",
                "obligation_ids",
                "resource_keys",
            ],
            "执行任务",
        )?;
        let id = string(item, "id", "执行任务")?.to_owned();
        let state = string(item, "state", "执行任务")?;
        if !matches!(state, "planned" | "pending_resolution" | "unsupported") {
            return Err(format!("{id}: 非法任务状态"));
        }
        string(item, "adapter_id", "执行任务")?;
        for obligation in unique_strings(item, "obligation_ids", "执行任务")? {
            if !obligations.contains_key(obligation) {
                return Err(format!("{id}: 引用未知义务 {obligation}"));
            }
            covered.insert(obligation.to_owned());
        }
        unique_strings(item, "resource_keys", "执行任务")?;
        if tasks.insert(id.clone(), Vec::<String>::new()).is_some() {
            return Err(format!("重复任务 ID：{id}"));
        }
    }
    for item in array(root, "unresolved_conditions", "计划")? {
        let item = object(item, "待解析条件")?;
        keys(item, &["obligation_id", "reason"], "待解析条件")?;
        let id = string(item, "obligation_id", "待解析条件")?;
        string(item, "reason", "待解析条件")?;
        if !obligations.contains_key(id) {
            return Err(format!("待解析条件引用未知义务 {id}"));
        }
        covered.insert(id.to_owned());
    }
    for (id, (required, applicability)) in &obligations {
        if *required && applicability != "not_applicable" && !covered.contains(id) {
            return Err(format!("必需义务没有任务或待解析条件：{id}"));
        }
    }

    for item in array(root, "edges", "计划")? {
        let item = object(item, "依赖边")?;
        keys(item, &["before", "after"], "依赖边")?;
        let before = string(item, "before", "依赖边")?;
        let after = string(item, "after", "依赖边")?;
        if before == after || !tasks.contains_key(before) || !tasks.contains_key(after) {
            return Err(format!("无效任务依赖：{before} → {after}"));
        }
        tasks
            .get_mut(before)
            .expect("已检查任务存在")
            .push(after.to_owned());
    }
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    for id in tasks.keys() {
        visit(id, &tasks, &mut visited, &mut active)?;
    }

    Ok(CheckPlan {
        request_id,
        plan_id,
        obligation_ids: obligations.into_keys().collect(),
        task_ids: tasks.into_keys().collect(),
        document,
    })
}

fn visit(
    id: &str,
    tasks: &BTreeMap<String, Vec<String>>,
    visited: &mut BTreeSet<String>,
    active: &mut BTreeSet<String>,
) -> Result<(), String> {
    if visited.contains(id) {
        return Ok(());
    }
    if !active.insert(id.to_owned()) {
        return Err(format!("任务依赖成环：{id}"));
    }
    for next in &tasks[id] {
        visit(next, tasks, visited, active)?;
    }
    active.remove(id);
    visited.insert(id.to_owned());
    Ok(())
}

fn validate_identities(root: &Map<String, Value>) -> Result<(), String> {
    let content = object(
        root.get("content_identity").ok_or("缺内容身份")?,
        "内容身份",
    )?;
    keys(content, &["kind", "digest"], "内容身份")?;
    if !matches!(
        string(content, "kind", "内容身份")?,
        "working_tree" | "git_index" | "git_ref" | "ci"
    ) {
        return Err("未知内容身份类别".into());
    }
    digest(content, "digest", "内容身份")?;
    let policy = object(root.get("policy_identity").ok_or("缺政策身份")?, "政策身份")?;
    keys(policy, &["source", "revision", "digest"], "政策身份")?;
    string(policy, "source", "政策身份")?;
    string(policy, "revision", "政策身份")?;
    digest(policy, "digest", "政策身份")
}

fn version(root: &Map<String, Value>) -> Result<(), String> {
    let version = string(root, "schema_version", "计划")?;
    let Some((major, minor)) = version.split_once('.') else {
        return Err("计划协议版本格式错误".into());
    };
    if major != "1" || minor.is_empty() || minor.parse::<u32>().is_err() {
        return Err("未知计划协议版本".into());
    }
    Ok(())
}

fn extensions(value: Option<&Value>) -> Result<(), String> {
    if let Some(value) = value {
        let object = object(value, "extensions")?;
        if object
            .keys()
            .any(|key| !key.starts_with("x-") || key.len() < 3)
        {
            return Err("extensions 只能使用 x- 前缀".into());
        }
    }
    Ok(())
}

fn digest(object: &Map<String, Value>, key: &str, label: &str) -> Result<(), String> {
    let value = string(object, key, label)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label}.{key} 必须是小写 SHA-256"));
    }
    Ok(())
}

fn unique_strings<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<Vec<&'a str>, String> {
    let items = array(object, key, label)?;
    let mut found = BTreeSet::new();
    let mut output = Vec::with_capacity(items.len());
    for value in items {
        let text = value
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("{label}.{key} 必须为非空字符串数组"))?;
        if !found.insert(text) {
            return Err(format!("{label}.{key} 含重复项"));
        }
        output.push(text);
    }
    Ok(output)
}

fn array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.{key} 必须是数组"))
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
