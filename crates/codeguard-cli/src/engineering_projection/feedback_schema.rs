use super::schema_sources::{IDS, source};
use guard_contract::SchemaValidator;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn filename(reference: &str) -> Result<String, String> {
    let file = IDS
        .iter()
        .find(|(id, _)| *id == reference)
        .map_or(reference, |(_, name)| name);
    if source(file).is_none() {
        return Err("unsupported embedded Schema reference".into());
    }
    Ok(file.into())
}
fn pointer(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}
fn rewrite(value: &mut Value, own: &str, pending: &mut BTreeSet<String>) -> Result<(), String> {
    match value {
        Value::Object(obj) => {
            obj.remove("$id");
            if let Some(Value::String(reference)) = obj.get_mut("$ref") {
                let (file, fragment) = reference.split_once('#').unwrap_or((reference, ""));
                let file = if file.is_empty() {
                    own.into()
                } else {
                    filename(file)?
                };
                if !fragment.is_empty() && !fragment.starts_with('/') {
                    return Err("unsupported embedded Schema anchor".into());
                }
                pending.insert(file.clone());
                *reference = format!("#/$defs/{}{}", pointer(&file), fragment);
            }
            for (key, v) in obj {
                if key != "$ref" {
                    rewrite(v, own, pending)?;
                }
            }
        }
        Value::Array(items) => {
            for v in items {
                rewrite(v, own, pending)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// 对确切已编译历史Schema求值，所有引用先改写为内存本地引用，禁止外部I/O。
pub(super) fn validate(value: &Value, version: &str) -> Result<(), String> {
    let minor = version
        .strip_prefix("0.")
        .and_then(|s| s.strip_suffix(".0"))
        .ok_or("unknown feedback version")?;
    let root = format!("check-feedback-v0.{minor}.schema.json");
    if !(38..=73).any(|n| version == format!("0.{n}.0")) || source(&root).is_none() {
        return Err("unknown feedback version".into());
    }
    validate_root(value, &root)
}
// 测试真实装配器的中止wire；公共结果投影仍不支持check_aborted，不由此授信。
#[cfg(test)]
pub(super) fn validate_aborted(value: &Value) -> Result<(), String> {
    validate_root(value, "check-aborted-v0.23.schema.json")
}
fn validate_root(value: &Value, root: &str) -> Result<(), String> {
    let root = root.to_owned();
    let mut pending = BTreeSet::from([root.clone()]);
    let mut done = BTreeMap::new();
    while let Some(name) = pending.pop_first() {
        if done.contains_key(&name) {
            continue;
        }
        if done.len() >= 128 {
            return Err("Schema resource budget exceeded".into());
        }
        let mut schema: Value = serde_json::from_slice(source(&name).ok_or("unknown Schema")?)
            .map_err(|e| e.to_string())?;
        rewrite(&mut schema, &name, &mut pending)?;
        done.insert(name, schema);
    }
    let defs: Map<String, Value> = done.into_iter().collect();
    let schema = json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$ref":format!("#/$defs/{}",pointer(&root)),"$defs":defs});
    let validator =
        SchemaValidator::build(&serde_json::to_vec(&schema).map_err(|e| e.to_string())?)?;
    let issues = validator.validate(value);
    if issues.is_empty() {
        Ok(())
    } else {
        // 只输出编译期 Schema 中的固定位置，不回显实例路径、错误值或原件正文。
        // 根 oneOf 的摘要会遮住具体字段；逐个求值既有闭合分支，仅用于诊断。
        let mut paths: BTreeSet<String> =
            issues.into_iter().map(|issue| issue.schema_path).collect();
        let root_pointer = format!("/$defs/{}", pointer(&root));
        let branches = schema
            .pointer(&format!("{root_pointer}/oneOf"))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for index in 0..branches.min(4) {
            let mut branch = schema.clone();
            branch["$ref"] = json!(format!("#{root_pointer}/oneOf/{index}"));
            let validator =
                SchemaValidator::build(&serde_json::to_vec(&branch).map_err(|e| e.to_string())?)?;
            paths.extend(
                validator
                    .validate(value)
                    .into_iter()
                    .map(|issue| issue.schema_path),
            );
        }
        Err(format!(
            "feedback does not satisfy its exact closed Schema; static_schema_paths={}",
            paths.into_iter().take(24).collect::<Vec<_>>().join(",")
        ))
    }
}
