use sha2::{Digest, Sha256};

const RULE: &[u8] = include_bytes!("../../../rulepacks/go/required_package.json");

/// 返回固定内置Go整文件package规则摘要；不代表精度或发行资格已验收。
#[must_use]
pub fn go_package_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}

/// 解释同轮固定Go grammar的完整根节点事实，返回是否需原生确认的缺声明候选。
/// 参数明确语言、完整文件模式、根/子节点类型、存在事实和截断状态；
/// 返回None表示范围或事实不完整，Some(false)仅表示本规则没有候选，不代表源码合法。
#[must_use]
pub fn missing_go_package_candidate(
    language: &str,
    whole_file: bool,
    root_syntax_kind: &str,
    child_syntax_kind: &str,
    present: bool,
    truncated: bool,
) -> Option<bool> {
    // 由同一固定规则读取解释条件；不把相同根类型的Rust/Java等套入Go规则。
    let rule: serde_json::Value = serde_json::from_slice(RULE).ok()?;
    if language != rule["language"].as_str()?
        || !whole_file
        || rule["source_scope"] != "whole_file"
        || root_syntax_kind != rule["root_kind"].as_str()?
        || child_syntax_kind != rule["required_direct_named_child_kind"].as_str()?
        || truncated
    {
        return None;
    }
    Some(!present)
}
