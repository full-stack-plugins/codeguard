//! 原生 print-config 中单规则的有界解析；不解释JS配置或授权规则策略。
use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, Visitor},
};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};

/// ESLint原生print-config输出的规则投影；其它配置字段不进入对话。
#[derive(Deserialize)]
struct EffectiveConfig {
    #[serde(deserialize_with = "unique_rules")]
    rules: BTreeMap<String, Value>,
}
fn unique_rules<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Value>, D::Error> {
    /// 有界规则映射读者，拒绝重复规则键导致的歧义。
    struct RulesVisitor;
    impl<'de> Visitor<'de> for RulesVisitor {
        type Value = BTreeMap<String, Value>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("规则对象且键不可重复")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut rules = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, Value>()? {
                if rules.len() >= 100_000
                    || key.is_empty()
                    || key.len() > 512
                    || key.chars().any(char::is_control)
                    || rules.insert(key, value).is_some()
                {
                    return Err(A::Error::custom("规则键重复或超过预算"));
                }
            }
            Ok(rules)
        }
    }
    deserializer.deserialize_map(RulesVisitor)
}
/// 解析原生有效配置中指定规则的严重度；返回None表示该规则未出现。
/// 参数为有界原生stdout字节与规则ID；坏结构、重复规则或非法级别返回未完成原因。
pub fn parse_eslint_effective_rule(bytes: &[u8], rule: &str) -> Result<Option<u8>, &'static str> {
    if bytes.len() > 1024 * 1024
        || bytes.is_empty()
        || rule.is_empty()
        || rule.len() > 512
        || rule.chars().any(char::is_control)
    {
        return Err("eslint_effective_settings_invalid");
    }
    if std::str::from_utf8(bytes)
        .ok()
        .is_some_and(|text| text.trim() == "undefined")
    {
        return Err("eslint_effective_target_not_selected");
    }
    let config: EffectiveConfig =
        serde_json::from_slice(bytes).map_err(|_| "eslint_effective_settings_invalid")?;
    let Some(value) = config.rules.get(rule) else {
        return Ok(None);
    };
    let value = match value {
        Value::Array(values) => values.first().ok_or("eslint_effective_settings_invalid")?,
        value => value,
    };
    match (value.as_u64(), value.as_str()) {
        (Some(level), _) if level <= 2 => Ok(Some(level as u8)),
        (_, Some("off")) => Ok(Some(0)),
        (_, Some("warn")) => Ok(Some(1)),
        (_, Some("error")) => Ok(Some(2)),
        _ => Err("eslint_effective_settings_invalid"),
    }
}
