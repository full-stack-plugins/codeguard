//! 工作区文档规则的候选来源关联；不计算 Cargo 成员归属或有效 lint 覆盖。
use crate::{CheckerConfiguration, inspect_cargo_documentation_config};

/// 关联显式继承成员与一份已绑定摘要的工作区候选清单。
/// 参数是成员/候选清单字节、成员构建根和两份来源；无 workspace 返回 None 以继续祖先搜索。
/// 返回配置始终 unknown；显式 package.workspace、非法数据或缺规则均不能推断生效。
pub fn inspect_cargo_documentation_workspace(
    member: &[u8],
    candidate: &[u8],
    build_root: &str,
    source: &str,
    workspace_source: &str,
) -> Option<Vec<CheckerConfiguration>> {
    let mut rows = inspect_cargo_documentation_config(Some(member), build_root, source);
    if !rows
        .iter()
        .all(|r| r.reason == "cargo_doc_lints_workspace_inheritance_unresolved")
    {
        return Some(rows);
    }
    let parsed_member = parse(member)?;
    if parsed_member
        .get("package")
        .and_then(|p| p.get("workspace"))
        .is_some()
    {
        for row in &mut rows {
            row.reason = "cargo_doc_lints_explicit_workspace_reference_unresolved".into();
        }
        return Some(rows);
    }
    let Some(parsed) = parse(candidate) else {
        for row in &mut rows {
            row.reason = "cargo_doc_lints_workspace_manifest_unavailable_or_invalid".into();
        }
        return Some(rows);
    };
    let workspace = parsed.get("workspace")?;
    let Some(workspace) = workspace.as_table() else {
        for row in &mut rows {
            row.reason = "cargo_doc_lints_workspace_manifest_unavailable_or_invalid".into();
        }
        return Some(rows);
    };
    let Some(lints) = workspace.get("lints") else {
        for row in &mut rows {
            row.reason = "cargo_doc_lints_workspace_rules_not_declared".into();
        }
        return Some(rows);
    };
    // 仅复用精确等级解析器观察原 workspace.lints，不制造成员归属或生效证明。
    let mut projection = parsed_member;
    projection
        .as_table_mut()?
        .insert("lints".into(), lints.clone());
    rows = super::cargo_documentation_config::inspect_workspace_projection(
        projection, build_root, source,
    );
    for row in &mut rows {
        if row.reason == "cargo_doc_lints_declared_scope_unverified" {
            row.reason = "cargo_doc_lints_workspace_declared_scope_unverified".into();
        }
        row.next_action = format!(
            "候选workspace清单：{workspace_source}；成员继承声明：{source}；成员归属/排除/通配及完整生效范围待原Cargo核验；{}",
            row.next_action
        );
    }
    Some(rows)
}

fn parse(bytes: &[u8]) -> Option<toml::Value> {
    if bytes.len() > 256 * 1024 {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::inspect_cargo_documentation_workspace;

    #[test]
    fn root_package_inheritance_and_invalid_workspace_lints_stay_unknown() {
        let root = b"[package]\nname='root'\nversion='0.1.0'\n[lints]\nworkspace=true\n[workspace]\n[workspace.lints.rustdoc]\nbroken_intra_doc_links='deny'\n";
        let rows =
            inspect_cargo_documentation_workspace(root, root, ".", "Cargo.toml", "Cargo.toml")
                .unwrap();
        assert_eq!(
            rows[0].reason,
            "cargo_doc_lints_workspace_declared_scope_unverified"
        );
        assert_eq!(rows[0].configuration, "unknown");
        assert!(rows[0].next_action.contains("broken_intra_doc_links=deny"));
        let invalid = b"[workspace]\n[workspace.lints.clippy]\nmissing_errors_doc={level='warn',priority='x'}\n";
        let rows =
            inspect_cargo_documentation_workspace(root, invalid, ".", "Cargo.toml", "Cargo.toml")
                .unwrap();
        assert_eq!(rows[1].reason, "cargo_doc_lints_declaration_invalid");
        assert_eq!(rows[1].configuration, "unknown");
        assert!(
            inspect_cargo_documentation_workspace(
                root,
                b"[package]\nname='unrelated'\n",
                ".",
                "Cargo.toml",
                "Cargo.toml"
            )
            .is_none()
        );
    }
}
