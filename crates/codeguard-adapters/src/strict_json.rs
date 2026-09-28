//! 有界调用者使用的JSON读者；所有层级拒绝重复键。
use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;
/// 递归JSON投影，兼作读者；调用者负责原始字节预算。
struct StrictJson(Value);
impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(Self(Value::Null)).map(Self)
    }
}
impl<'de> Visitor<'de> for StrictJson {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("无重复键的有界JSON")
    }
    fn visit_unit<E: Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_f64<E: Error>(self, v: f64) -> Result<Value, E> {
        Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("非法数值"))
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.into()))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(v) = a.next_element::<StrictJson>()? {
            if items.len() >= 100_000 {
                return Err(A::Error::custom("数组预算超限"));
            }
            items.push(v.0);
        }
        Ok(Value::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut items = Map::new();
        while let Some((k, v)) = a.next_entry::<String, StrictJson>()? {
            if items.len() >= 100_000 || items.insert(k, v.0).is_some() {
                return Err(A::Error::custom("对象预算超限或重复键"));
            }
        }
        Ok(Value::Object(items))
    }
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    serde_json::from_slice::<StrictJson>(bytes).map(|v| v.0)
}

/// 有界JSON报告解析，递归拒绝重复对象字段；返回值仍不具有来源或策略权威。
/// 参数为原始报告字节，最多16MiB；错误表示报告不能被完整消费。
pub fn parse_unique_json(bytes: &[u8]) -> Result<Value, &'static str> {
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        return Err("json_size_invalid");
    }
    parse(bytes).map_err(|_| "json_fields_invalid")
}
