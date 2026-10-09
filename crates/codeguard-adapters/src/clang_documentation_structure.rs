//! 原生函数文档结构事实的封闭形状及源码定位校验，不重建AST或签发语义准确性。
use serde_json::Value;
use std::collections::BTreeSet;

fn exact(value: &Value, fields: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && fields.iter().all(|f| o.contains_key(*f)))
}

/// 核验原生结构观察；参数为结构对象与可选冻结源码，返回形状/组件一致且定位匹配与否。
/// 无源码时只核验封闭协议；调用者仍须原工具/源码身份和消费收据，不以本函数证明执行或完整覆盖。
pub fn valid_clang_documentation_structure(value: &Value, source: Option<&[u8]>) -> bool {
    if !exact(
        value,
        &[
            "schema_version",
            "observation_type",
            "authority",
            "qualification",
            "coverage_proven",
            "functions",
            "unresolved_declaration_kinds",
            "semantic_accuracy",
            "errors_and_behavior",
        ],
    ) || value["schema_version"] != "0.1.0"
        || value["observation_type"] != "clang_function_documentation_structure"
        || value["authority"] != "local_unverified"
        || value["qualification"] != "not_granted"
        || value["coverage_proven"] != false
        || value["semantic_accuracy"] != "not_evaluated"
        || value["errors_and_behavior"] != "not_evaluated"
    {
        return false;
    }
    let Some(rows) = value["functions"].as_array().filter(|r| r.len() <= 2000) else {
        return false;
    };
    let Some(unresolved) = value["unresolved_declaration_kinds"]
        .as_array()
        .filter(|r| r.len() <= 512)
    else {
        return false;
    };
    if unresolved.iter().any(|v| {
        !v.as_str().is_some_and(|s| {
            s.ends_with("Decl") && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric())
        })
    }) {
        return false;
    }
    let text = match source {
        Some(s) => match std::str::from_utf8(s) {
            Ok(s) if s.len() <= 1024 * 1024 => Some(s),
            _ => return false,
        },
        None => None,
    };
    let starts = text.map(|text| {
        let bytes = text.as_bytes();
        let mut starts = vec![0usize];
        let mut i = 0;
        while i < bytes.len() {
            if matches!(bytes[i], b'\r' | b'\n') {
                if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    i += 1;
                }
                starts.push(i + 1);
            }
            i += 1;
        }
        starts
    });
    let mut offsets = BTreeSet::new();
    for row in rows {
        if !exact(
            row,
            &[
                "name",
                "offset_byte",
                "line",
                "column_byte",
                "comment_presence",
                "purpose_description",
                "parameters",
                "return_description",
                "missing_components",
                "structure_status",
            ],
        ) {
            return false;
        }
        let Some(name) = row["name"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        else {
            return false;
        };
        let Some(offset) = row["offset_byte"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
        else {
            return false;
        };
        if !offsets.insert(offset)
            || !row["line"].as_u64().is_some_and(|n| n > 0)
            || !row["column_byte"].as_u64().is_some_and(|n| n > 0)
        {
            return false;
        }
        if let Some(text) = text {
            if text.get(offset..offset.saturating_add(name.len())) != Some(name) {
                return false;
            }
            let starts = starts.as_ref().expect("冻结源码行索引");
            let index = starts
                .partition_point(|start| *start <= offset)
                .saturating_sub(1);
            let line = (index + 1) as u64;
            let column = (offset - starts[index] + 1) as u64;
            if row["line"] != line || row["column_byte"] != column {
                return false;
            }
        }
        let Some(params) = row["parameters"].as_array().filter(|p| p.len() <= 256) else {
            return false;
        };
        let Some(missing) = row["missing_components"].as_array() else {
            return false;
        };
        let mut expected = Vec::<String>::new();
        match (
            row["structure_status"].as_str(),
            row["comment_presence"].as_str(),
        ) {
            (Some("unknown_redeclaration"), Some("unknown_redeclaration"))
            | (Some("unresolved_comment_structure"), Some("present")) => {
                if row["purpose_description"] != "unknown"
                    || row["return_description"] != "unknown"
                    || !params.is_empty()
                {
                    return false;
                }
            }
            (Some("observed_supported_subset"), Some("absent")) => {
                if row["purpose_description"] != "unknown"
                    || row["return_description"] != "unknown"
                    || !params.is_empty()
                {
                    return false;
                }
                expected.push("documentation_comment".into());
            }
            (Some("observed_supported_subset"), Some("present")) => {
                match row["purpose_description"].as_str() {
                    Some("missing") => expected.push("purpose".into()),
                    Some("nonempty") => {}
                    _ => return false,
                }
                for (index, param) in params.iter().enumerate() {
                    if !exact(param, &["index", "name", "description"])
                        || param["index"].as_u64() != Some(index as u64)
                    {
                        return false;
                    }
                    match param["description"].as_str() {
                        Some("unknown_unnamed") if param["name"].is_null() => {}
                        Some("missing" | "nonempty") => {
                            let Some(name) = param["name"].as_str().filter(|s| {
                                !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
                            }) else {
                                return false;
                            };
                            if param["description"] == "missing" {
                                expected.push(format!("parameter:{name}"));
                            }
                        }
                        _ => return false,
                    }
                }
                match row["return_description"].as_str() {
                    Some("missing") => expected.push("return".into()),
                    Some("unknown" | "nonempty" | "not_applicable") => {}
                    _ => return false,
                }
            }
            _ => return false,
        }
        if missing.len() != expected.len()
            || missing
                .iter()
                .zip(expected)
                .any(|(a, b)| a.as_str() != Some(b.as_str()))
        {
            return false;
        }
    }
    true
}
