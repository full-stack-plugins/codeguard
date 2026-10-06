use sha2::{Digest, Sha256};
const RULE: &[u8] = include_bytes!("../../../rulepacks/erlang/form_terminator.json");

/// 固定Erlang直接函数form终止符规则摘要；不赋予原生或语言资格权威。
#[must_use]
pub fn erlang_form_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}
