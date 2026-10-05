use serde_json::Value;
use sha2::{Digest, Sha256};

const RULE: &[u8] = include_bytes!("../../../rulepacks/python/required_suite.json");

/// 返回固定内置Python suite规则的字节摘要；不代表该规则已获质量或白名单批准。
#[must_use]
pub fn python_suite_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}

/// 判断父节点是否在内置规则的必须具有语句的suite范围。
/// 参数是固定Python grammar中的父节点类型；未知父节点不猜成违规。
#[must_use]
pub fn is_required_python_suite_parent(parent: &str) -> bool {
    serde_json::from_slice::<Value>(RULE)
        .ok()
        .is_some_and(|rule| {
            rule["required_parent_kinds"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == parent))
        })
}
