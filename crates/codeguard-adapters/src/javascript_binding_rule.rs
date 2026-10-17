use sha2::{Digest, Sha256};
const RULE: &[u8] =
    include_bytes!("../../../rulepacks/javascript/duplicate_direct_lexical_binding.json");

/// 返回内置JavaScript直接绑定候选规则配置摘要；不提供工具、规则或关闭批准。
#[must_use]
pub fn javascript_binding_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}

/// 读取固定规则的四种AST节点类型；坏规则配置拒绝而不猜默认值。
pub fn javascript_binding_node_kinds() -> Result<[String; 4], &'static str> {
    let rule: serde_json::Value =
        serde_json::from_slice(RULE).map_err(|_| "javascript_binding_rule_invalid")?;
    if rule["rule_id"] != "codeguard.javascript.duplicate_direct_lexical_binding"
        || rule["rule_version"] != "1.0.0"
        || rule["qualification"] != "candidate_unqualified"
    {
        return Err("javascript_binding_rule_invalid");
    }
    let read = |key| {
        rule[key]
            .as_str()
            .filter(|kind| {
                !kind.is_empty() && kind.len() <= 128 && !kind.chars().any(char::is_control)
            })
            .map(str::to_owned)
            .ok_or("javascript_binding_rule_invalid")
    };
    Ok([
        read("root_kind")?,
        read("declaration_kind")?,
        read("declarator_kind")?,
        read("name_kind")?,
    ])
}
