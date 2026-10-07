//! 固定Clang原生AST的函数文档结构观察；非空不证明语义准确，也不冒充原生警告。
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// 解析冻结源码对应的Clang JSON AST，返回文档存在性、描述结构和未解析声明。
/// 参数为有界原报告及UTF-8源码；来源/版本/前后稳定性必须由调用者核验，格式或关联矛盾返回错误。
pub fn parse_clang_documentation_ast(raw: &[u8], source: &[u8]) -> Result<Value, &'static str> {
    if raw.len() > 8 * 1024 * 1024 || source.len() > 1024 * 1024 {
        return Err("clang_documentation_ast_budget_exceeded");
    }
    let text = std::str::from_utf8(source).map_err(|_| "clang_documentation_source_invalid")?;
    let ast: Value = serde_json::from_slice(raw).map_err(|_| "clang_documentation_ast_invalid")?;
    if ast["kind"] != "TranslationUnitDecl" {
        return Err("clang_documentation_ast_invalid");
    }
    let mut stack = vec![(&ast, 0usize, false)];
    let mut visited = 0;
    let mut functions = Vec::new();
    let mut unresolved = BTreeSet::new();
    while let Some((node, depth, class_context)) = stack.pop() {
        visited += 1;
        if visited > 50_000 || depth > 64 {
            return Err("clang_documentation_ast_budget_exceeded");
        }
        let kind = node["kind"]
            .as_str()
            .ok_or("clang_documentation_ast_invalid")?;
        if node["isImplicit"] == true {
            continue;
        }
        let ordinary_method = matches!(kind, "CXXMethodDecl" | "CXXConstructorDecl")
            && class_context
            && node["name"].as_str().is_some_and(|name| {
                !name.is_empty()
                    && name.bytes().enumerate().all(|(index, byte)| {
                        byte.is_ascii_alphabetic()
                            || byte == b'_'
                            || (index > 0 && byte.is_ascii_digit())
                    })
            });
        if kind == "FunctionDecl" || ordinary_method {
            if functions.len() >= 2000 {
                return Err("clang_documentation_ast_budget_exceeded");
            }
            functions.push(function(node, text)?);
            continue;
        }
        if matches!(
            kind,
            "TranslationUnitDecl" | "NamespaceDecl" | "LinkageSpecDecl" | "CXXRecordDecl"
        ) {
            // 类本身的契约仍未核验；只扩展有明确类上下文的普通方法与显式构造函数。
            if kind == "CXXRecordDecl" {
                unresolved.insert(kind.to_owned());
            }
            for child in children(node)?.iter().rev() {
                stack.push((child, depth + 1, kind == "CXXRecordDecl"));
            }
        } else if kind.ends_with("Decl")
            && !matches!(kind, "TypedefDecl" | "UsingDirectiveDecl" | "EmptyDecl")
        {
            unresolved.insert(kind.to_owned());
        }
    }
    Ok(
        json!({"schema_version":"0.1.0","observation_type":"clang_function_documentation_structure",
        "authority":"local_unverified","qualification":"not_granted","coverage_proven":false,
        "functions":functions,"unresolved_declaration_kinds":unresolved,
        "semantic_accuracy":"not_evaluated","errors_and_behavior":"not_evaluated"}),
    )
}

fn children(node: &Value) -> Result<&[Value], &'static str> {
    match node.get("inner") {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        _ => Err("clang_documentation_ast_invalid"),
    }
}

fn nonempty_text(node: &Value, depth: usize) -> Result<bool, &'static str> {
    if depth > 64 {
        return Err("clang_documentation_ast_budget_exceeded");
    }
    if node["kind"] == "TextComment" {
        return Ok(!node["text"]
            .as_str()
            .ok_or("clang_documentation_ast_invalid")?
            .trim()
            .is_empty());
    }
    let mut nonempty = false;
    for child in children(node)? {
        nonempty |= nonempty_text(child, depth + 1)?;
    }
    Ok(nonempty)
}

fn supported_comment(node: &Value, depth: usize) -> Result<bool, &'static str> {
    if depth > 64 {
        return Err("clang_documentation_ast_budget_exceeded");
    }
    let supported = match node["kind"].as_str() {
        Some("FullComment" | "ParagraphComment" | "TextComment" | "ParamCommandComment") => true,
        Some("BlockCommandComment") => matches!(
            node["name"].as_str(),
            Some("brief" | "short" | "details" | "return" | "returns" | "result")
        ),
        _ => false,
    };
    let mut all = supported;
    for child in children(node)? {
        all &= supported_comment(child, depth + 1)?;
    }
    Ok(all)
}

fn function(node: &Value, source: &str) -> Result<Value, &'static str> {
    let name = node["name"]
        .as_str()
        .filter(|n| !n.is_empty())
        .ok_or("clang_documentation_ast_invalid")?;
    let loc = &node["loc"];
    let offset = loc["offset"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("clang_documentation_ast_source_mismatch")?;
    let len = loc["tokLen"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("clang_documentation_ast_source_mismatch")?;
    if loc.get("spellingLoc").is_some()
        || loc.get("expansionLoc").is_some()
        || loc.get("includedFrom").is_some()
        || loc.get("file").is_some_and(|f| f != "<stdin>")
        || len != name.len()
        || source.get(offset..offset.saturating_add(len)) != Some(name)
    {
        return Err("clang_documentation_ast_source_mismatch");
    }
    let parts = children(node)?;
    let params: Vec<&Value> = parts
        .iter()
        .filter(|p| p["kind"] == "ParmVarDecl")
        .collect();
    if params.len() > 256 {
        return Err("clang_documentation_ast_budget_exceeded");
    }
    let comments: Vec<&Value> = parts
        .iter()
        .filter(|p| p["kind"] == "FullComment")
        .collect();
    if comments.len() > 1 {
        return Err("clang_documentation_ast_invalid");
    }
    let inherited = node.get("previousDecl").is_some();
    let presence = if inherited {
        "unknown_redeclaration"
    } else if comments.is_empty() {
        "absent"
    } else {
        "present"
    };
    let mut missing = Vec::new();
    let mut parameter_rows = Vec::new();
    let mut purpose = "unknown";
    let mut return_description = "unknown";
    if !inherited && comments.is_empty() {
        missing.push("documentation_comment".to_owned());
    }
    let supported = comments
        .first()
        .map(|comment| supported_comment(comment, 0))
        .transpose()?
        .unwrap_or(true);
    if !inherited && !comments.is_empty() && supported {
        let sections = children(comments[0])?;
        let mut purpose_present = false;
        for section in sections {
            if section["kind"] == "ParagraphComment"
                || (section["kind"] == "BlockCommandComment"
                    && matches!(
                        section["name"].as_str(),
                        Some("brief" | "short" | "details")
                    ))
            {
                purpose_present |= nonempty_text(section, 0)?;
            }
        }
        purpose = if purpose_present {
            "nonempty"
        } else {
            "missing"
        };
        if !purpose_present {
            missing.push("purpose".to_owned());
        }
        for section in sections
            .iter()
            .filter(|p| p["kind"] == "ParamCommandComment")
        {
            let index = section["paramIdx"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("clang_documentation_parameter_unresolved")?;
            let param = params
                .get(index)
                .ok_or("clang_documentation_parameter_unresolved")?;
            if param["name"] != section["param"] || !param["name"].is_string() {
                return Err("clang_documentation_parameter_unresolved");
            }
        }
        for (index, param) in params.iter().enumerate() {
            let param_name = param["name"].as_str();
            let mut described = false;
            for section in sections.iter().filter(|p| {
                p["kind"] == "ParamCommandComment" && p["paramIdx"].as_u64() == Some(index as u64)
            }) {
                described |= nonempty_text(section, 0)?;
            }
            let state = if param_name.is_none() {
                "unknown_unnamed"
            } else if described {
                "nonempty"
            } else {
                "missing"
            };
            if state == "missing" {
                missing.push(format!("parameter:{}", param_name.unwrap_or("")));
            }
            parameter_rows.push(json!({"index":index,"name":param_name,"description":state}));
        }
        // 只认明确的内置返回类型；typedef、函数指针及复杂C++返回不能按字符串猜测void语义。
        let qual = node["type"]["qualType"]
            .as_str()
            .ok_or("clang_documentation_ast_invalid")?;
        let ret = qual.split_once(" (").map(|(ret, _)| ret);
        if ret == Some("void") {
            return_description = "not_applicable";
        } else if matches!(
            ret,
            Some(
                "bool"
                    | "_Bool"
                    | "char"
                    | "signed char"
                    | "unsigned char"
                    | "short"
                    | "unsigned short"
                    | "int"
                    | "unsigned int"
                    | "long"
                    | "unsigned long"
                    | "long long"
                    | "unsigned long long"
                    | "float"
                    | "double"
                    | "long double"
            )
        ) {
            let mut described = false;
            for section in sections.iter().filter(|p| {
                p["kind"] == "BlockCommandComment"
                    && matches!(p["name"].as_str(), Some("return" | "returns" | "result"))
            }) {
                described |= nonempty_text(section, 0)?;
            }
            return_description = if described { "nonempty" } else { "missing" };
            if !described {
                missing.push("return".to_owned());
            }
        }
    }
    let prefix = &source.as_bytes()[..offset];
    let (mut line, mut column, mut i) = (1, 1, 0);
    while i < prefix.len() {
        if matches!(prefix[i], b'\r' | b'\n') {
            line += 1;
            column = 1;
            if i + 1 < prefix.len() && matches!((prefix[i], prefix[i + 1]), (b'\r', b'\n')) {
                i += 1;
            }
        } else {
            column += 1;
        }
        i += 1;
    }
    Ok(
        json!({"name":name,"offset_byte":offset,"line":line,"column_byte":column,
        "comment_presence":presence,"purpose_description":purpose,"parameters":parameter_rows,
        "return_description":return_description,"missing_components":missing,
        "structure_status":if inherited {"unknown_redeclaration"}else if !supported {"unresolved_comment_structure"}else{"observed_supported_subset"}}),
    )
}
