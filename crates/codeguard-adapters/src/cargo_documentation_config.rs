//! Cargo 文档 lint 的有界静态声明观察；不替代 Cargo/rustc 生效规则解析。
use crate::CheckerConfiguration;

/// 观察本构建根的五项精确文档规则，返回两个独立原检查器的未知配置投影。
/// 参数为可读清单字节、构建根及清单引用；继承/源码属性/组覆盖尚未解析，不授予配置完整性。
pub fn inspect_cargo_documentation_config(
    bytes: Option<&[u8]>,
    build_root: &str,
    source: &str,
) -> Vec<CheckerConfiguration> {
    let parsed = bytes
        .filter(|b| b.len() <= 256 * 1024)
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|s| s.parse::<toml::Value>().ok());
    inspect_parsed(parsed, build_root, source)
}

/// 复用精确声明观察器处理已核对来源的 workspace.lints 投影，不授予生效状态。
pub(super) fn inspect_workspace_projection(
    parsed: toml::Value,
    build_root: &str,
    source: &str,
) -> Vec<CheckerConfiguration> {
    inspect_parsed(Some(parsed), build_root, source)
}

fn inspect_parsed(
    parsed: Option<toml::Value>,
    build_root: &str,
    source: &str,
) -> Vec<CheckerConfiguration> {
    let rustdoc_rules = [
        ("rust", "missing_docs"),
        ("rustdoc", "broken_intra_doc_links"),
    ];
    let clippy_rules = [
        ("clippy", "missing_errors_doc"),
        ("clippy", "missing_panics_doc"),
        ("clippy", "missing_safety_doc"),
    ];
    let mut result = Vec::new();
    for (checker, rules) in [
        ("rust.cargo_rustdoc", rustdoc_rules.as_slice()),
        ("rust.cargo_clippy", clippy_rules.as_slice()),
    ] {
        let mut declarations = Vec::new();
        let reason = match parsed.as_ref().and_then(toml::Value::as_table) {
            None => "cargo_doc_lints_manifest_unavailable_or_invalid",
            Some(root) if !root.get("package").is_some_and(toml::Value::is_table) => {
                if !root.contains_key("package")
                    && root.get("workspace").is_some_and(toml::Value::is_table)
                {
                    "cargo_doc_lints_virtual_workspace_unresolved"
                } else {
                    "cargo_doc_lints_manifest_unavailable_or_invalid"
                }
            }
            Some(root) => match root.get("lints") {
                None => "cargo_doc_lints_not_declared_in_manifest",
                Some(value) => match value.as_table() {
                    None => "cargo_doc_lints_declaration_invalid",
                    Some(lints) if lints.contains_key("workspace") => {
                        if lints["workspace"].as_bool() == Some(true) && lints.len() == 1 {
                            "cargo_doc_lints_workspace_inheritance_unresolved"
                        } else {
                            "cargo_doc_lints_declaration_invalid"
                        }
                    }
                    Some(lints) => {
                        let mut invalid = false;
                        for (tool, rule) in rules {
                            let Some(table) = lints.get(*tool) else {
                                continue;
                            };
                            let Some(table) = table.as_table() else {
                                invalid = true;
                                continue;
                            };
                            let Some(value) = table.get(*rule) else {
                                continue;
                            };
                            let level = value.as_str().or_else(|| {
                                value
                                    .as_table()
                                    .and_then(|t| t.get("level"))
                                    .and_then(toml::Value::as_str)
                            });
                            if let Some(level) =
                                level.filter(|l| matches!(*l, "allow" | "warn" | "deny" | "forbid"))
                            {
                                declarations.push(format!("{rule}={level}"));
                            } else {
                                invalid = true;
                            }
                            if let Some(t) = value.as_table() {
                                if t.keys().any(|k| k != "level" && k != "priority")
                                    || t.get("priority").is_some_and(|v| v.as_integer().is_none())
                                {
                                    invalid = true;
                                }
                            }
                        }
                        if invalid {
                            "cargo_doc_lints_declaration_invalid"
                        } else if !declarations.is_empty() {
                            "cargo_doc_lints_declared_scope_unverified"
                        } else {
                            "cargo_doc_lints_group_or_default_unresolved"
                        }
                    }
                },
            },
        };
        result.push(CheckerConfiguration {
                build_root:build_root.into(),checker_id:checker.into(),category:"comments".into(),
                configuration:"unknown".into(),configuration_ref:source.into(),reason:reason.into(),
                next_action:format!("原清单精确声明：{}；按原Cargo核验工作区继承、源码属性、lint组/priority与目标范围；未声明不等于未启用，不添加规则或修改无关源码，不把空章节或零诊断当详细文档合格",if declarations.is_empty(){"未解析".into()}else{declarations.join("、")}),
            });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::inspect_cargo_documentation_config;
    #[test]
    fn malformed_dynamic_and_oversized_declarations_remain_unknown() {
        for input in [
            None,
            Some(b"\xff".as_slice()),
            Some(b"[package".as_slice()),
            Some(vec![b' '; 256 * 1024 + 1].as_slice()),
        ] {
            let r = inspect_cargo_documentation_config(input, ".", "Cargo.toml");
            assert!(r.iter().all(|c| c.configuration == "unknown"
                && c.reason == "cargo_doc_lints_manifest_unavailable_or_invalid"));
        }
        for tail in [
            "[lints]\nworkspace=false",
            "[lints]\nworkspace=true\n[lints.clippy]\nmissing_errors_doc='warn'",
            "[lints.clippy]\nmissing_errors_doc='${LEVEL}'",
            "[lints.clippy]\nmissing_errors_doc={level='warn',priority='x'}",
            "[lints.clippy]\nmissing_errors_doc={level='warn',typo=true}",
            "[lints.clippy]\nmissing_errors_doc=5",
        ] {
            let manifest = format!("[package]\nname='a'\nversion='0.1.0'\n{tail}\n");
            let r =
                inspect_cargo_documentation_config(Some(manifest.as_bytes()), ".", "Cargo.toml");
            assert_eq!(r[1].configuration, "unknown");
            assert_eq!(r[1].reason, "cargo_doc_lints_declaration_invalid");
        }
    }
}
