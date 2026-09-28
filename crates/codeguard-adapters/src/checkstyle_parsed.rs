use crate::CheckstyleDiagnostic;
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::HashSet;

/// Checkstyle XML 结构观察；即使结构有效也不证明本轮执行、规则加载或范围覆盖。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckstyleParsed {
    /// 局部报告结构及声明版本有效，不是检查通过。
    pub report_valid: bool,
    /// 字段可验证的原生诊断候选；结构局部异常时仍保留证据。
    pub diagnostics: Vec<CheckstyleDiagnostic>,
    /// 报告列出的文件，包括零诊断文件。
    pub files: Vec<String>,
    /// exception 或根级无文件事件数量，不当作源码规则违规。
    pub processing_errors: usize,
    /// 稳定未完成原因，不包含原生私有消息。
    pub reason: Option<&'static str>,
}

/// 解析有界 UTF-8 Checkstyle XML。
/// 参数为原始报告字节与独立预期版本；返回局部观察，不授予门禁、白名单或覆盖权威。
#[must_use]
pub fn parse_checkstyle_xml(bytes: &[u8], expected_version: &str) -> CheckstyleParsed {
    let mut result = CheckstyleParsed {
        report_valid: false,
        diagnostics: Vec::new(),
        files: Vec::new(),
        processing_errors: 0,
        reason: Some("invalid_checkstyle_report"),
    };
    if bytes.len() > 16 * 1024 * 1024 {
        result.reason = Some("report_too_large");
        return result;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return result;
    };
    let Ok(doc) = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            entity_resolver: None,
        },
    ) else {
        return result;
    };
    let root = doc.root_element();
    if !named(root, "checkstyle") {
        return result;
    }
    let mut valid = !expected_version.trim().is_empty()
        && root.attribute("version") == Some(expected_version)
        && attributes(root, &["version"])
        && !nonblank_text(root);
    let mut seen = HashSet::new();
    for file in root.children().filter(Node::is_element) {
        if !named(file, "file") {
            valid = false;
            if named(file, "exception") || named(file, "error") {
                result.processing_errors += 1;
            }
            continue;
        }
        let Some(filename) = file.attribute("name").filter(|v| !v.trim().is_empty()) else {
            valid = false;
            continue;
        };
        valid &= attributes(file, &["name"]) && !nonblank_text(file);
        if !seen.insert(filename) {
            valid = false;
        } else {
            result.files.push(filename.to_owned());
        }
        for event in file.children().filter(Node::is_element) {
            if named(event, "exception") {
                result.processing_errors += 1;
                valid = false;
            } else if let Some(diagnostic) = diagnostic(event, filename) {
                result.diagnostics.push(diagnostic);
            } else {
                valid = false;
            }
        }
    }
    result.report_valid = valid;
    result.reason = if valid {
        None
    } else {
        Some("checkstyle_report_incomplete")
    };
    result
}

fn named(node: Node<'_, '_>, name: &str) -> bool {
    node.tag_name().name() == name && node.tag_name().namespace().is_none()
}

fn attributes(node: Node<'_, '_>, names: &[&str]) -> bool {
    node.attributes()
        .all(|a| a.namespace().is_none() && names.contains(&a.name()))
}

fn nonblank_text(node: Node<'_, '_>) -> bool {
    node.children()
        .any(|n| n.is_text() && n.text().is_some_and(|t| !t.trim().is_empty()))
}

fn diagnostic(node: Node<'_, '_>, filename: &str) -> Option<CheckstyleDiagnostic> {
    if !named(node, "error")
        || !attributes(node, &["line", "column", "severity", "message", "source"])
        || node.children().any(|n| n.is_element())
        || nonblank_text(node)
    {
        return None;
    }
    let line = number(node.attribute("line")?)?;
    let column = match node.attribute("column") {
        Some(value) => Some(number(value)?),
        None => None,
    };
    if column == Some(0) {
        return None;
    }
    let severity = node.attribute("severity")?;
    if !matches!(severity, "error" | "warning" | "info") {
        return None;
    }
    let source = node.attribute("source").filter(|v| !v.trim().is_empty())?;
    let message = node.attribute("message").filter(|v| !v.trim().is_empty())?;
    Some(CheckstyleDiagnostic {
        filename: filename.to_owned(),
        source: source.to_owned(),
        line,
        column,
        severity: severity.to_owned(),
        message: message.to_owned(),
    })
}

fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|v| v.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
