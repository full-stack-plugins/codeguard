use serde_json::{Value, json};
use std::{
    path::{Component, Path},
    sync::atomic::AtomicBool,
    time::Instant,
};
use sha2::{Digest, Sha256};

/// 按已确定的项目上下文观察冻结Rust文件，输出可持久化脱敏原生证据。
/// 参数含根、相对文件、显式已选工具与共同预算；不运行Cargo/源码或提供关闭批准。
pub(crate) fn observe(
    root: &Path,
    path: &str,
    tool: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut native = json!({"status":"incomplete","reason":"rustfmt_tool_not_found","version":null,"tool_sha256":null,"edition":null,"observation_kind":"formatter_parser","diagnostics":[],"edition_context":null});
    if let Some(tool) = tool {
        let project = crate::rust_project_syntax::observe(root, path, tool, deadline, cancelled);
        if project["native"].is_object() {
            native = project["native"].clone();
        } else {
            native["reason"] = project["reason"].clone();
        }
        native["edition_context"] = project["edition_context"].clone();
        native["edition"] = project["edition_context"]["edition"].clone();
    } else {
        match crate::rust_project_edition::RustProjectEdition::capture(root, path) {
            Ok(context) => {
                native["edition"] = json!(context.edition);
                native["edition_context"] = context.report();
            }
            Err(reason) => native["reason"] = json!(reason),
        }
    }
    native
}

/// 校验Rust原生证据固定字段、上下文来源与源码行号；不核验完整项目能力或批准。
pub(crate) fn valid(native: &Value, source: Option<&[u8]>) -> bool {
    if !exact(
        native,
        &[
            "status",
            "reason",
            "version",
            "tool_sha256",
            "edition",
            "observation_kind",
            "diagnostics",
            "edition_context",
        ],
    ) || native["observation_kind"] != "formatter_parser"
        || !(native["version"].is_null() || native["version"] == "rustfmt 1.9.0-stable")
        || !(native["tool_sha256"].is_null()
            || native["tool_sha256"].as_str().is_some_and(valid_sha))
        || !(native["edition"].is_null() || native["edition"].as_str().is_some_and(edition))
        || !native["reason"].as_str().is_some_and(|s| {
            !s.is_empty() && s.len() <= 96 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
    {
        return false;
    }
    let Some(rows) = native["diagnostics"]
        .as_array()
        .filter(|rows| rows.len() <= 32)
    else {
        return false;
    };
    if rows.iter().any(|row| {
        !exact(row, &["line", "rule_id"])
            || row["rule_id"] != "rust.syntax"
            || !row["line"].as_u64().is_some_and(|line| {
                line > 0
                    && line <= u32::MAX as u64
                    && source.is_none_or(|bytes| {
                        std::str::from_utf8(bytes)
                            .is_ok_and(|s| line as usize <= s.split('\n').count())
                    })
            })
    }) {
        return false;
    }
    let context = &native["edition_context"];
    if !context.is_null()
        && (!exact(
            context,
            &[
                "edition",
                "manifest_ref",
                "manifest_sha256",
                "workspace_manifest_ref",
                "workspace_manifest_sha256",
            ],
        ) || context["edition"] != native["edition"]
            || !context["edition"].as_str().is_some_and(edition)
            || !context["manifest_ref"].as_str().is_some_and(manifest_path)
            || !context["manifest_sha256"].as_str().is_some_and(valid_sha)
            || !((context["workspace_manifest_ref"].is_null()
                && context["workspace_manifest_sha256"].is_null())
                || (context["workspace_manifest_ref"]
                    .as_str()
                    .is_some_and(manifest_path)
                    && context["workspace_manifest_sha256"]
                        .as_str()
                        .is_some_and(valid_sha))))
    {
        return false;
    }
    match native["status"].as_str() {
        Some("incomplete") => rows.is_empty(),
        Some("completed") => {
            rows.is_empty()
                && native["reason"] == "rustfmt_native_parse_no_diagnostics"
                && complete(native)
        }
        Some("diagnostics_observed") => {
            !rows.is_empty()
                && native["reason"] == "rustfmt_native_parse_diagnostics"
                && complete(native)
        }
        _ => false,
    }
}
fn complete(native: &Value) -> bool {
    native["version"] == "rustfmt 1.9.0-stable"
        && native["tool_sha256"].as_str().is_some_and(valid_sha)
        && native["edition_context"].is_object()
}

/// 保存或投影前核对适用edition声明来源；参数为根、相对文件及原观察。
pub(crate) fn context_current(root: &Path, path: &str, native: &Value) -> bool {
    match crate::rust_project_edition::RustProjectEdition::capture(root, path) {
        Ok(context) => context.report() == native["edition_context"],
        Err(reason) => {
            native["edition_context"].is_null()
                && native["status"] == "incomplete"
                && native["reason"] == reason
        }
    }
}
/// 校验原生首次或复检任务证据；参数包含target/tool/native，历史源码变化不改写原证据。
pub(crate) fn valid_evidence(root: &Path, evidence: &Value) -> bool {
    let Some(path) = evidence["target"]["path"].as_str() else {
        return false;
    };
    if path.is_empty()
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || !Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        || Path::new(path).extension().is_none_or(|ext| ext != "rs")
    {
        return false;
    }
    let current = crate::plain_syntax_source::read_plain_source(&root.join(path))
        .ok()
        .filter(|bytes| {
            evidence["target"]["source_sha256"] == format!("{:x}", Sha256::digest(bytes))
        });
    evidence["target"]["language"] == "rust"
        && exact(&evidence["target"], &["path", "language", "source_sha256"])
        && (evidence["target"]["source_sha256"].is_null()
            || evidence["target"]["source_sha256"]
                .as_str()
                .is_some_and(valid_sha))
        && (evidence["tool_path"].is_null()
            || evidence["tool_path"]
                .as_str()
                .is_some_and(|p| Path::new(p).is_absolute()))
        && (!complete(&evidence["native"])
            || (evidence["tool_path"].is_string()
                && evidence["target"]["source_sha256"].is_string()))
        && valid(&evidence["native"], current.as_deref())
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn edition(s: &str) -> bool {
    matches!(s, "2015" | "2018" | "2021" | "2024")
}
fn manifest_path(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('\\')
        && !s.chars().any(char::is_control)
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && Path::new(s).file_name().is_some_and(|n| n == "Cargo.toml")
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
