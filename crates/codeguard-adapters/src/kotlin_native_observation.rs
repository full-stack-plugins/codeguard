//! Kotlin 历史观察验证；与当前源码相同时核对原生 UTF-16 与 UTF-8 字节坐标。
use serde_json::Value;
use std::collections::BTreeSet;

/// 校验局部观察字段、工具版本、原因及位置；历史记录不授予修复或交付权威。
/// 参数为原生观察及可选的当前同摘要源码；返回是否符合固定 Kotlin 协议。
pub fn valid_kotlin_native_observation(native: &Value, current: Option<&[u8]>) -> bool {
    let keys = [
        "status",
        "reason",
        "version",
        "tool_sha256",
        "tool_identity_scope",
        "diagnostics",
        "context_diagnostics",
    ];
    if !native
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || native["tool_identity_scope"] != "launcher_only"
        || !(native["version"].is_null() || native["version"] == "kotlinc-jvm 2.4.10")
        || !(native["tool_sha256"].is_null()
            || native["tool_sha256"].as_str().is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }))
        || !native["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
    {
        return false;
    }
    let (Some(syntax), Some(context)) = (
        native["diagnostics"].as_array(),
        native["context_diagnostics"].as_array(),
    ) else {
        return false;
    };
    if syntax.len() + context.len() > 32 {
        return false;
    }
    let verified = native["version"] == "kotlinc-jvm 2.4.10" && native["tool_sha256"].is_string();
    match native["status"].as_str() {
        Some("completed")
            if verified
                && syntax.is_empty()
                && context.is_empty()
                && native["reason"] == "kotlin_native_single_file_no_diagnostics" => {}
        Some("diagnostics_observed")
            if verified
                && !syntax.is_empty()
                && context.is_empty()
                && native["reason"] == "kotlin_native_syntax_diagnostics" => {}
        Some("incomplete")
            if (syntax.is_empty()
                && context.is_empty()
                && native["reason"] != "kotlin_project_context_unresolved")
                || (verified
                    && !context.is_empty()
                    && native["reason"] == "kotlin_project_context_unresolved") => {}
        _ => return false,
    }
    let mut seen = BTreeSet::new();
    syntax
        .iter()
        .map(|r| (r, false))
        .chain(context.iter().map(|r| (r, true)))
        .all(|(r, is_context)| {
            let Some(rule) = r["rule_id"].as_str() else {
                return false;
            };
            let Some(line) = r["line"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            else {
                return false;
            };
            let Some(byte) = r["column_byte"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= 1024 * 1024 + 1)
            else {
                return false;
            };
            let Some(utf16) = r["column_utf16"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= 1024 * 1024 + 1)
            else {
                return false;
            };
            r.as_object().is_some_and(|o| {
                o.len() == 4
                    && ["rule_id", "line", "column_byte", "column_utf16"]
                        .iter()
                        .all(|k| o.contains_key(*k))
            }) && if is_context {
                rule.strip_prefix("kotlin.context.").is_some_and(|s| {
                    !s.is_empty()
                        && s.len() <= 128
                        && s != "SYNTAX"
                        && s.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                })
            } else {
                rule == "kotlin.syntax"
            } && seen.insert((rule.to_owned(), line, byte))
                && current.is_none_or(|bytes| {
                    std::str::from_utf8(bytes)
                        .ok()
                        .and_then(|s| s.split('\n').nth((line - 1) as usize))
                        .is_some_and(|s| {
                            let s = s.trim_end_matches('\r');
                            let end = (byte - 1) as usize;
                            s.get(..end).is_some_and(|prefix| {
                                prefix.encode_utf16().count() as u64 + 1 == utf16
                            })
                        })
                })
        })
}
