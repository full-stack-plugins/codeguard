use crate::CheckstyleRuleBinding;
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeMap;
use std::collections::HashSet;

/// 按 10.21.4 原配置生成精确 source 到检查类与修复方向的局部绑定。
/// 参数为原配置字节及独立版本；未知/歧义上下文返回 None，不猜自定义 ID 的含义。
#[must_use]
pub fn checkstyle_comment_rule_bindings(
    bytes: &[u8],
    version: &str,
) -> Option<BTreeMap<String, CheckstyleRuleBinding>> {
    if version != "10.21.4" || !checkstyle_comment_config_local_eligible(bytes) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let doc = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: 10_000,
            entity_resolver: None,
        },
    )
    .ok()?;
    let mut bindings = BTreeMap::new();
    for node in doc.descendants().filter(Node::is_element) {
        let name =
            canonical_module_name(node.attribute("name").unwrap_or_default()).unwrap_or_default();
        let (summary, rule_reference, step) = match name {
            "MissingJavadocType" => (
                "类型缺少 Javadoc",
                "https://checkstyle.org/checks/javadoc/missingjavadoctype.html",
                "确认类型的用途与原配置范围后补充 Javadoc，不编造业务语义",
            ),
            "MissingJavadocMethod" => (
                "方法或构造器缺少 Javadoc",
                "https://checkstyle.org/checks/javadoc/missingjavadocmethod.html",
                "确认方法/构造器实际行为，补充用途和契约；先复核继承及生成代码归属",
            ),
            "JavadocType" => (
                "类型文档标签需要核对",
                "https://checkstyle.org/checks/javadoc/javadoctype.html",
                "按原配置核对泛型参数、record 组件与 @param、@author、@version 及未知标签；作者和版本依据项目已有事实填写，不编造身份或版本，不修改格式规则来逃避复检",
            ),
            "JavadocVariable" => (
                "字段或枚举常量缺少 Javadoc",
                "https://checkstyle.org/checks/javadoc/javadocvariable.html",
                "按原配置范围核对字段或枚举常量的实际用途并补充 Javadoc；不编造含义，不改忽略规则",
            ),
            "JavadocMethod" => (
                "方法文档标签需要核对",
                "https://checkstyle.org/checks/javadoc/javadocmethod.html",
                "核对注释归属以及参数、返回值和异常标签；继承/同一行声明等边界先复现",
            ),
            _ => continue,
        };
        if node.tag_name().name() != "module" {
            continue;
        }
        let checker_class = format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{name}Check");
        let native_source = node
            .children()
            .filter(Node::is_element)
            .find(|n| n.attribute("name") == Some("id"))
            .and_then(|n| n.attribute("value"))
            .map(str::to_owned)
            .unwrap_or_else(|| checker_class.clone());
        let binding = CheckstyleRuleBinding {
            native_source: native_source.clone(),
            checker_class,
            summary,
            rule_reference,
            repair_steps: vec![
                step,
                "使用同一原配置和原检查器复检；零诊断不单独作为项目交付或任务关闭条件",
            ],
        };
        if bindings.insert(native_source, binding).is_some() {
            return None;
        }
    }
    Some(bindings)
}

/// 识别当前可直接快照执行的静态注释配置。
/// 参数是原始配置字节；返回 false 表示需解析完整资源/模块上下文，不替换用户规则。
#[must_use]
pub fn checkstyle_comment_config_local_eligible(bytes: &[u8]) -> bool {
    const DTD: &str = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\">";
    if bytes.len() > 1024 * 1024 {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    // 只认可锁定版本已映射到制品内部的 PUBLIC 标识；禁止内部实体及未知外部 DTD。
    if text.matches("<!DOCTYPE").count() != 1
        || !text.contains(DTD)
        || text.contains("<!ENTITY")
        || text.contains("${")
    {
        return false;
    }
    let Ok(doc) = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: 10_000,
            entity_resolver: None,
        },
    ) else {
        return false;
    };
    let root = doc.root_element();
    if !module(root, "Checker") || !unique_properties(root) {
        return false;
    }
    let mut trees = 0;
    let mut checks = 0;
    let mut sources = HashSet::new();
    for child in root.children().filter(Node::is_element) {
        if property(child) {
            continue;
        }
        if !module(child, "TreeWalker") || !unique_properties(child) {
            return false;
        }
        trees += 1;
        for check in child.children().filter(Node::is_element) {
            if property(check) {
                continue;
            }
            if !(module(check, "MissingJavadocType")
                || module(check, "JavadocMethod")
                || module(check, "JavadocType")
                || module(check, "MissingJavadocMethod")
                || module(check, "JavadocVariable"))
            {
                return false;
            }
            if !check.children().filter(Node::is_element).all(property) || !unique_properties(check)
            {
                return false;
            }
            // 10.21.4 有 id 时只报告 id，否则报告完整检查类；相同 source 无法独立归属。
            let identity = check
                .children()
                .filter(Node::is_element)
                .find(|n| n.attribute("name") == Some("id"))
                .and_then(|n| n.attribute("value"))
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    format!(
                        "com.puppycrawl.tools.checkstyle.checks.javadoc.{}Check",
                        canonical_module_name(check.attribute("name").unwrap_or_default())
                            .unwrap_or_default()
                    )
                });
            if !sources.insert(identity) {
                return false;
            }
            checks += 1;
        }
    }
    trees == 1
        && checks > 0
        && !doc
            .descendants()
            .any(|n| n.is_text() && n.text().is_some_and(|t| !t.trim().is_empty()))
}

fn unique_properties(module: Node<'_, '_>) -> bool {
    let mut seen = HashSet::new();
    module
        .children()
        .filter(Node::is_element)
        .filter(|n| n.tag_name().name() == "property")
        .all(|n| n.attribute("name").is_some_and(|name| seen.insert(name)))
}

fn module(n: Node<'_, '_>, name: &str) -> bool {
    n.tag_name().name() == "module"
        && n.tag_name().namespace().is_none()
        && n.attribute("name").and_then(canonical_module_name) == Some(name)
        && n.attributes()
            .all(|a| a.name() == "name" && a.namespace().is_none())
}

fn property(n: Node<'_, '_>) -> bool {
    n.tag_name().name() == "property"
        && n.tag_name().namespace().is_none()
        && n.attributes()
            .all(|a| matches!(a.name(), "name" | "value") && a.namespace().is_none())
        && !n.children().any(|c| c.is_element())
        && match (n.attribute("name"), n.attribute("value")) {
            (Some("id"), Some(v)) => {
                !v.is_empty()
                    && v.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            }
            (Some("severity"), Some("error" | "warning" | "info")) => true,
            (
                Some("scope" | "excludeScope"),
                Some("public" | "protected" | "package" | "private"),
            ) => n.parent_element().is_some_and(|p| {
                module(p, "JavadocVariable")
                    || module(p, "MissingJavadocMethod")
                    || module(p, "MissingJavadocType")
                    || module(p, "JavadocType")
            }),
            (Some("tokens"), Some(v)) => {
                v.len() <= 256
                    && n.parent_element().is_some_and(|p| {
                        v.split(',').map(str::trim).all(|token| {
                            if module(p, "JavadocVariable") {
                                matches!(token, "VARIABLE_DEF" | "ENUM_CONSTANT_DEF")
                            } else if module(p, "MissingJavadocMethod")
                                || module(p, "JavadocMethod")
                            {
                                matches!(
                                    token,
                                    "METHOD_DEF"
                                        | "CTOR_DEF"
                                        | "COMPACT_CTOR_DEF"
                                        | "ANNOTATION_FIELD_DEF"
                                )
                            } else if module(p, "MissingJavadocType") || module(p, "JavadocType") {
                                matches!(
                                    token,
                                    "CLASS_DEF"
                                        | "INTERFACE_DEF"
                                        | "ENUM_DEF"
                                        | "ANNOTATION_DEF"
                                        | "RECORD_DEF"
                                )
                            } else {
                                false
                            }
                        })
                    })
            }
            (Some(name @ ("allowedAnnotations" | "skipAnnotations")), Some(v)) => {
                n.parent_element().is_some_and(|p| {
                    if name == "skipAnnotations" {
                        module(p, "MissingJavadocType")
                    } else {
                        module(p, "MissingJavadocMethod")
                            || module(p, "JavadocMethod")
                            || module(p, "JavadocType")
                    }
                }) && v.len() <= 4096
                    && (v.is_empty()
                        || v.split(',').map(str::trim).all(|name| {
                            !name.is_empty()
                                && name.split('.').all(|part| {
                                    let mut chars = part.chars();
                                    chars.next().is_some_and(|c| {
                                        c.is_alphabetic() || matches!(c, '_' | '$')
                                    }) && chars
                                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
                                })
                        }))
            }
            (Some("allowMissingPropertyJavadoc"), Some(v)) => {
                n.parent_element()
                    .is_some_and(|p| module(p, "MissingJavadocMethod"))
                    && (v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"))
            }
            (
                Some(
                    name @ ("allowMissingParamTags"
                    | "allowMissingReturnTag"
                    | "validateThrows"
                    | "allowUnknownTags"),
                ),
                Some(v),
            ) => {
                n.parent_element().is_some_and(|p| {
                    (module(p, "JavadocMethod") && name != "allowUnknownTags")
                        || (module(p, "JavadocType")
                            && matches!(name, "allowMissingParamTags" | "allowUnknownTags"))
                }) && (v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"))
            }
            (Some("accessModifiers"), Some(v)) => {
                n.parent_element()
                    .is_some_and(|p| module(p, "JavadocMethod"))
                    && v.len() <= 256
                    && v.split(',').map(str::trim).all(|modifier| {
                        matches!(modifier, "public" | "protected" | "package" | "private")
                    })
            }
            (Some("minLineCount"), Some(v)) => {
                n.parent_element()
                    .is_some_and(|p| module(p, "MissingJavadocMethod"))
                    && v.trim().parse::<i32>().is_ok()
            }
            (Some("authorFormat" | "versionFormat"), Some(v)) => {
                n.parent_element().is_some_and(|p| module(p, "JavadocType"))
                    && !v.is_empty()
                    && v.len() <= 4096
                    && !v.chars().any(char::is_control)
            }
            (Some("ignoreMethodNamesRegex"), Some(v)) => {
                n.parent_element()
                    .is_some_and(|p| module(p, "MissingJavadocMethod"))
                    && !v.is_empty()
                    && v.len() <= 4096
                    && !v.chars().any(char::is_control)
            }
            (Some("ignoreNamePattern"), Some(v)) => {
                n.parent_element()
                    .is_some_and(|p| module(p, "JavadocVariable"))
                    && !v.is_empty()
                    && v.len() <= 4096
                    && !v.chars().any(char::is_control)
            }
            _ => false,
        }
}

// 只使用固定版本已知官方名称；不对未知包名做后缀截断或模糊匹配。
fn canonical_module_name(name: &str) -> Option<&'static str> {
    match name {
        "Checker" | "com.puppycrawl.tools.checkstyle.Checker" => Some("Checker"),
        "TreeWalker" | "com.puppycrawl.tools.checkstyle.TreeWalker" => Some("TreeWalker"),
        "MissingJavadocType"
        | "MissingJavadocTypeCheck"
        | "com.puppycrawl.tools.checkstyle.checks.javadoc.MissingJavadocTypeCheck" => {
            Some("MissingJavadocType")
        }
        "MissingJavadocMethod"
        | "MissingJavadocMethodCheck"
        | "com.puppycrawl.tools.checkstyle.checks.javadoc.MissingJavadocMethodCheck" => {
            Some("MissingJavadocMethod")
        }
        "JavadocType"
        | "JavadocTypeCheck"
        | "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocTypeCheck" => Some("JavadocType"),
        "JavadocMethod"
        | "JavadocMethodCheck"
        | "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocMethodCheck" => {
            Some("JavadocMethod")
        }
        "JavadocVariable"
        | "JavadocVariableCheck"
        | "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck" => {
            Some("JavadocVariable")
        }
        _ => None,
    }
}
