//! Node 包清单的纯解析；不读取文件、不启动包管理器。

use serde_json::Value;

/// 仅来自 package.json 字面字段的声明，不代表解析后依赖图。
#[derive(Debug, Eq, PartialEq)]
pub struct PackageDeclaration {
    /// 包名；未声明时为空。
    pub name: Option<String>,
    /// 包版本；未声明时为空。
    pub version: Option<String>,
}

/// 解析 JSON 清单中的字面声明；类型错误必须可见。
pub fn parse_package_json(bytes: &[u8]) -> Result<PackageDeclaration, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let object = root.as_object().ok_or("package.json 根节点不是对象")?;
    fn optional_nonempty_string(
        object: &serde_json::Map<String, Value>,
        key: &str,
    ) -> Result<Option<String>, String> {
        match object.get(key) {
            None => Ok(None),
            Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
            _ => Err(format!("package.json 字段 {key} 不是非空字符串")),
        }
    }
    Ok(PackageDeclaration {
        name: optional_nonempty_string(object, "name")?,
        version: optional_nonempty_string(object, "version")?,
    })
}

#[cfg(test)]
mod tests {
    use super::{PackageDeclaration, parse_package_json};

    #[test]
    fn accepts_only_literal_string_declarations() {
        assert_eq!(
            parse_package_json(br#"{"name":"demo","version":"1.2.3"}"#),
            Ok(PackageDeclaration {
                name: Some("demo".into()),
                version: Some("1.2.3".into())
            })
        );
        assert_eq!(
            parse_package_json(br#"{"private":true}"#),
            Ok(PackageDeclaration {
                name: None,
                version: None
            })
        );
    }

    #[test]
    fn rejects_malformed_and_wrong_type_values() {
        assert!(parse_package_json(b"not json").is_err());
        assert!(parse_package_json(br#"{"version":42}"#).is_err());
        assert!(parse_package_json(br#"{"version":""}"#).is_err());
    }
}
