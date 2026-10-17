use sha2::{Digest, Sha256};
const RULE: &[u8] =
    include_bytes!("../../../rulepacks/javascript/module_return_outside_function.json");

/// 返回模块函数外return候选规则配置摘要；不授予模块模式或违规权威。
#[must_use]
pub fn javascript_module_return_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}

/// 读取固定规则的根/return节点和函数边界；调用者必须已证明显式module模式。
/// 返回AST类型列表，损坏规则或模式不匹配时拒绝，不猜测默认语言语义。
pub fn javascript_module_return_node_kinds() -> Result<([String; 2], Vec<String>), &'static str> {
    let rule: serde_json::Value =
        serde_json::from_slice(RULE).map_err(|_| "javascript_module_return_rule_invalid")?;
    if rule["rule_id"] != "codeguard.javascript.module_return_outside_function"
        || rule["rule_version"] != "1.0.0"
        || rule["qualification"] != "candidate_unqualified"
        || rule["required_mode"] != "module"
    {
        return Err("javascript_module_return_rule_invalid");
    }
    let read = |value: &serde_json::Value| {
        value
            .as_str()
            .filter(|kind| {
                !kind.is_empty() && kind.len() <= 128 && !kind.chars().any(char::is_control)
            })
            .map(str::to_owned)
            .ok_or("javascript_module_return_rule_invalid")
    };
    let functions = rule["function_kinds"]
        .as_array()
        .filter(|rows| (1..=32).contains(&rows.len()))
        .ok_or("javascript_module_return_rule_invalid")?
        .iter()
        .map(read)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        [read(&rule["root_kind"])?, read(&rule["return_kind"])?],
        functions,
    ))
}
