//! PMD 6 原生 XML 报告的纯解析；不把 XML 结构正确误当官方 P3C 规则已加载。

use roxmltree::{Document, Node, ParsingOptions};

const PMD_NAMESPACE: &str = "http://pmd.sourceforge.net/report/2.0.0";
const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;

/// PMD 原生违规，规则归属与门禁严重度由已锁定执行上下文另行证明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PmdDiagnostic {
    /// 原生报告文件路径，尚未转换为受控项目路径。
    pub filename: String,
    /// 原生规则 ID。
    pub rule: String,
    /// 原生规则集文字，仅供诊断，不能证明 P3C 来源。
    pub ruleset: String,
    /// 一基起始行号。
    pub beginline: u32,
    /// 一基起始列号。
    pub begincolumn: u32,
    /// 一基结束行号。
    pub endline: u32,
    /// 一基结束列号。
    pub endcolumn: u32,
    /// PMD 原生优先级 1–5，策略层决定门禁影响。
    pub priority: u8,
    /// 原生诊断文本，公开前必须脱敏。
    pub message: String,
}

/// XML 报告结构与声明 PMD 版本的局部有效性。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PmdParseState {
    /// 仅证明 XML 结构可解析；不证明规则加载、执行范围或工具身份。
    ValidReport,
    /// 报告损坏、版本不符或原生处理失败；已解析诊断保留。
    Incomplete,
}

/// 原生报告解析结果；不得单独签发 P3C 或 Java lint 通过。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PmdParsed {
    /// 局部报告状态。
    pub state: PmdParseState,
    /// 能独立验证字段的源码违规候选。
    pub diagnostics: Vec<PmdDiagnostic>,
    /// 原生报告声明的文件列表；即使零违规也要核对扫描范围。
    pub files: Vec<String>,
    /// 原生处理错误数，不当作源码违规。
    pub processing_errors: usize,
    /// 原生 suppression 数，仅供后续策略核对。
    pub suppressed_count: usize,
    /// 未完成的稳定原因码。
    pub reason: Option<&'static str>,
}

/// 解析 PMD 6 XML 报告；拒绝 DTD、越界规模、未知结构和非法位置。
#[must_use]
pub fn parse_pmd_xml(bytes: &[u8], expected_pmd_version: &str) -> PmdParsed {
    if bytes.len() > MAX_REPORT_BYTES {
        return incomplete(Vec::new(), 0, 0, "report_too_large");
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return incomplete(Vec::new(), 0, 0, "invalid_xml_encoding");
    };
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: 100_000,
        entity_resolver: None,
    };
    let Ok(document) = Document::parse_with_options(text, options) else {
        return incomplete(Vec::new(), 0, 0, "invalid_xml_report");
    };
    let root = document.root_element();
    if !named(root, "pmd") {
        return incomplete(Vec::new(), 0, 0, "invalid_report_root");
    }
    let version_matches = root.attribute("version") == Some(expected_pmd_version);
    let mut diagnostics = Vec::new();
    let mut files = Vec::new();
    let mut processing_errors = 0;
    let mut suppressed_count = 0;
    let mut invalid = false;
    for child in root.children().filter(Node::is_element) {
        if named(child, "file") {
            let Some(filename) = child.attribute("name").filter(|name| !name.is_empty()) else {
                invalid = true;
                continue;
            };
            files.push(filename.to_owned());
            for entry in child.children().filter(Node::is_element) {
                if !named(entry, "violation") {
                    invalid = true;
                    continue;
                }
                match parse_violation(entry, filename) {
                    Some(violation) => diagnostics.push(violation),
                    None => invalid = true,
                }
            }
        } else if named(child, "error") || named(child, "configerror") {
            processing_errors += 1;
        } else if named(child, "suppressedviolation") {
            suppressed_count += 1;
        } else {
            invalid = true;
        }
    }
    let reason = if !version_matches {
        Some("pmd_version_mismatch")
    } else if processing_errors > 0 {
        Some("native_processing_error")
    } else if invalid {
        Some("invalid_report_entry")
    } else if suppressed_count > 0 {
        Some("native_suppression_requires_policy_review")
    } else {
        None
    };
    PmdParsed {
        state: if reason.is_some() {
            PmdParseState::Incomplete
        } else {
            PmdParseState::ValidReport
        },
        diagnostics,
        files,
        processing_errors,
        suppressed_count,
        reason,
    }
}

fn parse_violation(node: Node<'_, '_>, filename: &str) -> Option<PmdDiagnostic> {
    let rule = node.attribute("rule")?.trim();
    let ruleset = node.attribute("ruleset")?.trim();
    let message = node.text()?.trim();
    if rule.is_empty() || ruleset.is_empty() || message.is_empty() {
        return None;
    }
    let beginline = positive_number(node, "beginline")?;
    let begincolumn = positive_number(node, "begincolumn")?;
    let endline = positive_number(node, "endline")?;
    let endcolumn = positive_number(node, "endcolumn")?;
    if endline < beginline || (endline == beginline && endcolumn < begincolumn) {
        return None;
    }
    let priority = node.attribute("priority")?.parse::<u8>().ok()?;
    if !(1..=5).contains(&priority) {
        return None;
    }
    Some(PmdDiagnostic {
        filename: filename.to_owned(),
        rule: rule.to_owned(),
        ruleset: ruleset.to_owned(),
        beginline,
        begincolumn,
        endline,
        endcolumn,
        priority,
        message: message.to_owned(),
    })
}

fn positive_number(node: Node<'_, '_>, name: &str) -> Option<u32> {
    node.attribute(name)?
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
}

fn named(node: Node<'_, '_>, local_name: &str) -> bool {
    node.tag_name().namespace() == Some(PMD_NAMESPACE) && node.tag_name().name() == local_name
}

fn incomplete(
    diagnostics: Vec<PmdDiagnostic>,
    processing_errors: usize,
    suppressed_count: usize,
    reason: &'static str,
) -> PmdParsed {
    PmdParsed {
        state: PmdParseState::Incomplete,
        diagnostics,
        files: Vec::new(),
        processing_errors,
        suppressed_count,
        reason: Some(reason),
    }
}
