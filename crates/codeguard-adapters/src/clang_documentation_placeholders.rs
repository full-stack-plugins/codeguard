//! 从已关联的原生Clang AST观察完整说明仅为占位标记的情况，属于自有规则。
use crate::clang_documentation_ast::parse_clang_documentation_ast;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// 返回编译进当前制品的占位解析与核验实现身份；返回值为SHA-256摘要，无输入参数。
/// 同时绑定底层结构解析，避免策略实现升级仍复用旧输入上的失败预算。
pub fn clang_documentation_placeholder_engine_sha256() -> String {
    let mut hash = Sha256::new();
    hash.update(b"clang-documentation-placeholder-engine-v1\0");
    for source in [
        include_bytes!("clang_documentation_placeholders.rs").as_slice(),
        include_bytes!("clang_placeholder_validation.rs").as_slice(),
    ] {
        hash.update((source.len() as u64).to_le_bytes());
        hash.update(source);
    }
    hash.update(crate::clang_documentation_ast::clang_documentation_structure_engine_sha256());
    format!("{:x}", hash.finalize())
}

/// 观察用途、命名参数及适用返回说明中的明确占位标记。
/// 参数为有界原生AST和冻结源码；返回脱敏位置，错误沿用原生关联/预算错误。
/// 仅整个说明是标记时产生自有规则观察，不证明其他文本准确，不代替原工具身份核验。
pub fn parse_clang_documentation_placeholders(
    raw: &[u8],
    source: &[u8],
) -> Result<Value, &'static str> {
    let structure = parse_clang_documentation_ast(raw, source)?;
    let ast: Value = serde_json::from_slice(raw).map_err(|_| "clang_documentation_ast_invalid")?;
    let rows: BTreeMap<u64, &Value> = structure["functions"]
        .as_array()
        .ok_or("clang_documentation_ast_invalid")?
        .iter()
        .filter(|row| {
            row["structure_status"] == "observed_supported_subset"
                && row["comment_presence"] == "present"
        })
        .map(|row| {
            Ok((
                row["offset_byte"]
                    .as_u64()
                    .ok_or("clang_documentation_ast_invalid")?,
                row,
            ))
        })
        .collect::<Result<_, &'static str>>()?;
    let mut positions = Vec::new();
    let mut stack = vec![(&ast, 0usize)];
    let mut visited = 0usize;
    while let Some((node, depth)) = stack.pop() {
        visited += 1;
        if visited > 50_000 || depth > 64 {
            return Err("clang_documentation_ast_budget_exceeded");
        }
        if node["isImplicit"] == true {
            continue;
        }
        if let Some(row) = node["loc"]["offset"]
            .as_u64()
            .and_then(|offset| rows.get(&offset))
        {
            if node["name"] == row["name"]
                && matches!(
                    node["kind"].as_str(),
                    Some("FunctionDecl" | "CXXMethodDecl" | "CXXConstructorDecl")
                )
            {
                let comment = children(node)?
                    .iter()
                    .find(|child| child["kind"] == "FullComment")
                    .ok_or("clang_documentation_ast_invalid")?;
                let sections = children(comment)?;
                let mut components: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
                for section in sections {
                    let component = match section["kind"].as_str() {
                        Some("ParagraphComment") => Some("purpose".to_owned()),
                        Some("BlockCommandComment")
                            if matches!(
                                section["name"].as_str(),
                                Some("brief" | "short" | "details")
                            ) =>
                        {
                            Some("purpose".to_owned())
                        }
                        Some("BlockCommandComment")
                            if row["return_description"] == "nonempty"
                                && matches!(
                                    section["name"].as_str(),
                                    Some("return" | "returns" | "result")
                                ) =>
                        {
                            Some("return".to_owned())
                        }
                        Some("ParamCommandComment") => section["param"]
                            .as_str()
                            .map(|name| format!("parameter:{name}")),
                        _ => None,
                    };
                    if let Some(component) = component {
                        components.entry(component).or_default().push(section);
                    }
                }
                for (component, sections) in components {
                    let mut text = String::new();
                    for section in sections {
                        append_text(section, 0, &mut text)?;
                    }
                    // 限于整个说明匹配的固定标记；包含标记的正常句子不属于此规则。
                    let normalized = text
                        .trim()
                        .trim_end_matches(['.', '!', '?', '。', '！', '？'])
                        .trim()
                        .to_ascii_uppercase();
                    if matches!(
                        normalized.as_str(),
                        "TODO" | "TBD" | "FIXME" | "待补充" | "待完善"
                    ) {
                        positions.push(json!({"offset_byte":row["offset_byte"],"line":row["line"],"column_byte":row["column_byte"],"component":component}));
                    }
                }
                continue;
            }
        }
        // 与结构解析器相同的声明容器范围；不进入函数体或模板猜测其他声明。
        if matches!(
            node["kind"].as_str(),
            Some("TranslationUnitDecl" | "NamespaceDecl" | "LinkageSpecDecl" | "CXXRecordDecl")
        ) {
            for child in children(node)?.iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }
    Ok(
        json!({"schema_version":"0.1.0","observation_type":"clang_documentation_placeholder_observation","rule":"codeguard.documentation.placeholder_description","policy_version":"1","authority":"codeguard_structural_policy","qualification":"not_granted","coverage_proven":false,"semantic_accuracy":"not_evaluated","positions":positions}),
    )
}

fn children(node: &Value) -> Result<&[Value], &'static str> {
    match node.get("inner") {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        _ => Err("clang_documentation_ast_invalid"),
    }
}

fn append_text(node: &Value, depth: usize, text: &mut String) -> Result<(), &'static str> {
    if depth > 64 {
        return Err("clang_documentation_ast_budget_exceeded");
    }
    if node["kind"] == "TextComment" {
        text.push_str(
            node["text"]
                .as_str()
                .ok_or("clang_documentation_ast_invalid")?,
        );
        text.push(' ');
        if text.len() > 1024 * 1024 {
            return Err("clang_documentation_ast_budget_exceeded");
        }
    }
    for child in children(node)? {
        append_text(child, depth + 1, text)?;
    }
    Ok(())
}
