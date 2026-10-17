//! IDEA 风格 Markdown 内置格式化器（无外部依赖的替代实现）。
//!
//! 动机：codeguard 独立分发，用户与 CI 不一定装有 IntelliJ IDEA；
//! 但团队排版习惯来自 IDEA 的 ⌘⌥L。本模块在 Rust 内复现该风格，
//! 一致性由 golden 差分锁定——fixtures 里的 `.expected.md` 是
//! IDEA 2026.2.3 format.sh 的真实输出，测试断言逐字节一致。
//! IDEA 升级若改变风格，需在装有对应版本的机器上重新生成 golden。
//!
//! 已从 golden 提炼并实现的规则（V1）：
//! - 围栏代码块内容与围栏行原样
//! - ATX 标题：标记后多空格收敛为单空格；无空格标题（`###X`）原样；
//!   标题文本内部空格保留（不收拢）
//! - 正文/引用/列表内容：行内多空格收敛为单空格；行内代码 span 内容不动
//! - 列表：标记字符保留（不归一）；标记后多空格收敛单空格；缩进保留；
//!   同标记连续项之间不插空行，标记字符变化视为新块（块间一空行）
//! - 表格：列宽 = 各单元格显示宽度最大值（CJK 记 2）；
//!   数据行 `| 内容 + 填充 |`（两侧单空格 padding）；
//!   分隔行段为 `-` × (列宽+2)，无空格
//! - 块间距：异类型块之间恰一空行（无则插入、多余则收敛为一）；
//!   同块（嵌套列表、同标记列表）不加空行
//! - 行尾空白去除；尾部是否换行跟随输入
//!
//! 已知边界：与 IDEA 一样对语法损坏的 markdown 做容错排版
//! （无法区分「需重排」与「语法错误」，同退不合规），见档案 notes。

use serde_json::{Value, json};

/// 内置格式化器注册名（档案 formatter 字段使用）。
pub const FORMATTER_ID: &str = "builtin:idea-markdown";
/// 规则版本：随 golden 再生成而演进。
pub const RULES_VERSION: &str = "0.1.0-idea2026.2.3";

/// 对单个文件执行内置 check。
/// 返回 (status, reason)：("clean", None) / ("unformatted", None)。
pub fn check(content: &str) -> (&'static str, Option<&'static str>) {
    if format(content) == content {
        ("clean", None)
    } else {
        ("unformatted", None)
    }
}

/// 对单个文件执行内置 apply，返回格式化后的完整内容。
pub fn apply(content: &str) -> String {
    format(content)
}

/// 供报告投影的档案片段（工具身份如实标注内置与规则版本）。
pub(crate) fn tool_identity() -> Value {
    json!({
        "formatter": FORMATTER_ID,
        "tool_version": RULES_VERSION,
        "tool_digest": Value::Null,
        "builtin": true,
    })
}

// ---------- 格式化主体 ----------

/// 字符显示宽度（CJK 记 2，其余记 1；与 IDEA 表格对齐实测一致）。
fn char_width(c: char) -> usize {
    if (c as u32) >= 0x2E80 {
        2
    } else {
        1
    }
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// 行内文本收敛：多空格 → 单空格；反引号 code span 内容不动。
fn collapse_inline(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    let mut buffer = String::new();
    for c in line.chars() {
        if c == '`' {
            if !in_code {
                out.push_str(&collapse_spaces(&buffer));
                buffer.clear();
            } else {
                out.push_str(&buffer);
                buffer.clear();
            }
            out.push('`');
            in_code = !in_code;
        } else {
            buffer.push(c);
        }
    }
    if in_code {
        // 未闭合反引号：按普通文本处理已累积内容
        out.push_str(&collapse_spaces(&buffer));
    } else {
        out.push_str(&collapse_spaces(&buffer));
    }
    out
}

/// 收拢行内文本：内部多空格 → 单空格。
/// 保留段首/段尾的空白边界（各收敛为一个空格）——code span 与相邻
/// 文本之间的空格属于排版的一部分，不能吞掉。
fn collapse_spaces(s: &str) -> String {
    let leading = if s.starts_with(|c: char| c.is_whitespace()) {
        " "
    } else {
        ""
    };
    let trailing = if s.ends_with(|c: char| c.is_whitespace()) {
        " "
    } else {
        ""
    };
    let core = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(core.len() + 2);
    out.push_str(leading);
    out.push_str(&core);
    out.push_str(trailing);
    out
}

/// 逐行类型（用于块间距判定）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LineKind {
    Blank,
    Heading,
    /// 无序列表，携带标记字符
    List(char),
    /// 有序列表，携带分隔符（. 或 )）
    OrderedList(char),
    Table,
    Quote,
    Hr,
    Fence,
    Para,
}

fn classify(line: &str) -> LineKind {
    let t = line.trim_end();
    if t.trim().is_empty() {
        return LineKind::Blank;
    }
    if t.starts_with("```") || t.starts_with("~~~") {
        return LineKind::Fence;
    }
    if let Some(rest) = t.strip_prefix('#') {
        let hashes = 1 + rest.chars().take_while(|&c| c == '#').count();
        if hashes <= 6 {
            let after = &t[hashes..];
            // 无空格标题（###X）IDEA 原样保留，这里仍归类 Heading（间距同）
            return LineKind::Heading;
        }
    }
    if is_table_row(t) {
        return LineKind::Table;
    }
    if t.starts_with('>') {
        return LineKind::Quote;
    }
    if let Some((indent, body)) = split_indent(t) {
        if let Some(marker) = unordered_marker(body) {
            let _ = marker;
            return LineKind::List(marker);
        }
        if let Some((_, sep)) = ordered_marker(body) {
            return LineKind::OrderedList(sep);
        }
        if is_hr(body) {
            return LineKind::Hr;
        }
        let _ = indent;
    }
    LineKind::Para
}

fn split_indent(t: &str) -> Option<(&str, &str)> {
    let indent_len = t.len() - t.trim_start_matches([' ', '\t']).len();
    Some((&t[..indent_len], &t[indent_len..]))
}

fn unordered_marker(body: &str) -> Option<char> {
    let mut chars = body.chars();
    match chars.next() {
        Some(c @ ('*' | '+' | '-')) => {
            // 标记后必须跟空格才是列表项（`-` 单独成行是 HR）
            let after: String = chars.take(2).collect();
            if after.starts_with(' ') || after.starts_with('\t') {
                Some(c)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn ordered_marker(body: &str) -> Option<(String, char)> {
    let digits: String = body.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let mut rest = body[digits.len()..].chars();
    match rest.next() {
        Some(sep @ ('.' | ')')) => {
            let after: String = rest.take(2).collect();
            if after.starts_with(' ') || after.starts_with('\t') {
                Some((digits, sep))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_hr(body: &str) -> bool {
    let b = body.trim();
    (b == "---" || b == "***" || b == "___")
        || (b.len() >= 3
            && (b.chars().all(|c| c == '-')
                || b.chars().all(|c| c == '*')
                || b.chars().all(|c| c == '_')))
}

fn is_table_row(t: &str) -> bool {
    t.starts_with('|') && t.ends_with('|') && t.len() >= 2
}

fn is_table_separator(t: &str) -> bool {
    if !is_table_row(t) {
        return false;
    }
    t.trim_matches('|')
        .split('|')
        .all(|cell| {
            let c = cell.trim();
            !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
        })
}

/// 主入口：按 golden 规则格式化 markdown 文本。
pub(crate) fn format(input: &str) -> String {
    let ends_with_newline = input.ends_with('\n');
    let lines: Vec<&str> = input.split('\n').collect();
    // split 会在尾换行时产生末尾空串；重组时按原事实处理
    let has_trailing_empty = lines.last() == Some(&"");

    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let mut in_fence = false;
    let mut prev_kind: Option<LineKind> = None;
    let mut pending_blanks = 0usize;

    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim_end_matches([' ', '\t']);

        // 围栏代码块：内容与围栏行原样
        if in_fence {
            out.push(raw.to_string());
            if t.starts_with("```") || t.starts_with("~~~") {
                in_fence = false;
            }
            i += 1;
            prev_kind = Some(LineKind::Fence);
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            ensure_gap(&mut out, prev_kind, LineKind::Fence);
            out.push(raw.to_string());
            in_fence = true;
            i += 1;
            prev_kind = Some(LineKind::Fence);
            continue;
        }

        // 空行：延迟决定（由 ensure_gap/块逻辑消费）
        if t.trim().is_empty() {
            pending_blanks += 1;
            i += 1;
            continue;
        }

        let kind = classify(t);

        // 引用块前保留原有空行数（golden 实证 2→2；无空行时保底 1）
        if kind == LineKind::Quote && pending_blanks > 1 {
            if !out.is_empty() && !out.last().map(String::is_empty).unwrap_or(false) {
                out.push(String::new());
            }
            for _ in 1..pending_blanks {
                out.push(String::new());
            }
            pending_blanks = 0;
            prev_kind = Some(LineKind::Para); // 已完成间距，标记为已分隔
        }
        pending_blanks = 0;

        // 表格块：收集整个表格后统一重排
        if kind == LineKind::Table && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            ensure_gap(&mut out, prev_kind, LineKind::Table);
            let mut table: Vec<Vec<String>> = Vec::new();
            let mut j = i;
            while j < lines.len() && is_table_row(lines[j].trim_end()) {
                let cells: Vec<String> = lines[j]
                    .trim()
                    .trim_matches('|')
                    .split('|')
                    .map(|c| collapse_inline(c.trim()))
                    .collect();
                table.push(cells);
                j += 1;
            }
            for rendered in render_table(&table) {
                out.push(rendered);
            }
            i = j;
            prev_kind = Some(LineKind::Table);
            continue;
        }

        ensure_gap(&mut out, prev_kind, kind);

        match kind {
            LineKind::Heading => {
                let hashes = t.chars().take_while(|&c| c == '#').count();
                let after = &t[hashes..];
                if after.starts_with(' ') || after.starts_with('\t') {
                    let text = collapse_heading_tail(after);
                    out.push(format!("{} {}", "#".repeat(hashes), text));
                } else {
                    // 无空格标题原样
                    out.push(t.to_string());
                }
            }
            LineKind::Quote => {
                if t == ">" {
                    out.push(">".to_string());
                } else if let Some(rest) = t.strip_prefix('>') {
                    let rest = rest.strip_prefix(' ').unwrap_or(rest);
                    out.push(format!("> {}", collapse_inline(rest.trim())));
                } else {
                    out.push(collapse_inline(t));
                }
            }
            LineKind::List(_) => {
                let (indent, body) = split_indent(t).unwrap_or(("", t));
                let marker = body.chars().next().unwrap_or('*');
                let content = &body[1..];
                let content = content.strip_prefix(' ').unwrap_or(content);
                let content = content.strip_prefix('\t').unwrap_or(content);
                // 嵌套缩进的列表内容不再二次收拢内部结构，仅行内收敛
                out.push(format!("{}{} {}", indent, marker, collapse_inline(content.trim())));
            }
            LineKind::OrderedList(_) => {
                let (indent, body) = split_indent(t).unwrap_or(("", t));
                if let Some((digits, sep)) = ordered_marker(body) {
                    let used = digits.len() + 1;
                    let content = &body[used..];
                    let content = content.strip_prefix(' ').unwrap_or(content);
                    let content = content.strip_prefix('\t').unwrap_or(content);
                    out.push(format!("{}{}{} {}", indent, digits, sep, collapse_inline(content.trim())));
                } else {
                    out.push(collapse_inline(t));
                }
            }
            LineKind::Hr => out.push(t.to_string()),
            _ => out.push(collapse_inline(t)),
        }
        prev_kind = Some(kind);
        i += 1;
    }

    // 重组：块间单空行已由 ensure_gap 插入；补尾部
    let mut result = out.join("\n");
    if ends_with_newline && !has_trailing_empty {
        // 输入尾换行且最后一行非空：输出保持单尾换行
    } else if ends_with_newline && has_trailing_empty {
        // split 产生的末尾空串即尾换行的体现，join 后天然带尾换行
    }
    if ends_with_newline && !result.ends_with('\n') {
        result.push('\n');
    }
    if !ends_with_newline && result.ends_with('\n') {
        result.pop();
    }
    let _ = has_trailing_empty;
    result
}

/// 标题文本：保留内部空格（仅去掉首尾空白）。
fn collapse_heading_tail(after: &str) -> String {
    after.trim().to_string()
}

/// 块间距：异类型块之间恰一空行；同块不加。
/// 「同块」：同为无序列表且标记字符相同（嵌套缩进也视为同块延续）。
fn ensure_gap(out: &mut Vec<String>, prev: Option<LineKind>, next: LineKind) {
    let Some(prev) = prev else { return };
    let same_block = matches!(
        (prev, next),
        (LineKind::List(a), LineKind::List(b)) if a == b
    ) || matches!(
        (prev, next),
        (LineKind::OrderedList(a), LineKind::OrderedList(b)) if a == b
    ) || matches!(
        (prev, next),
        (LineKind::Quote, LineKind::Quote)
    );
    if same_block {
        return;
    }
    if !out.is_empty() && !out.last().map(String::is_empty).unwrap_or(false) {
        out.push(String::new());
    }
}

/// 表格渲染：列宽 = max 显示宽；数据行带 padding；分隔行 `-`×(w+2)。
fn render_table(rows: &[Vec<String>]) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0usize; cols];
    for (ri, row) in rows.iter().enumerate() {
        if ri == 1 {
            // 分隔行不参与列宽统计（其短横线长度是上一轮渲染的产物，
            // 计入会让表格每格式化一次就变宽一格——幂等性被破坏）
            continue;
        }
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(display_width(cell));
        }
    }
    let mut out = Vec::with_capacity(rows.len());
    for (ri, row) in rows.iter().enumerate() {
        if ri == 1 {
            // 分隔行：无空格
            let segs: Vec<String> = widths
                .iter()
                .map(|w| "-".repeat(w + 2))
                .collect();
            out.push(format!("|{}|", segs.join("|")));
            continue;
        }
        let cells: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(idx, w)| {
                let cell = row.get(idx).map(String::as_str).unwrap_or("");
                let pad = w.saturating_sub(display_width(cell));
                format!(" {}{}", cell, " ".repeat(pad))
            })
            .collect();
        out.push(format!("|{} |", cells.join(" |")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_marker_spaces_collapse_but_text_kept() {
        assert_eq!(format("#   一级   标题\n"), "# 一级   标题\n");
    }

    #[test]
    fn heading_without_space_stays() {
        assert_eq!(format("###三级\n"), "###三级\n");
    }

    #[test]
    fn paragraph_spaces_collapse() {
        assert_eq!(format("一些   正文   文本\n"), "一些 正文 文本\n");
    }

    #[test]
    fn inline_code_content_preserved() {
        assert_eq!(
            format("`行内代码   空格` 和 **加粗   间隔**。\n"),
            "`行内代码   空格` 和 **加粗 间隔**。\n"
        );
    }

    #[test]
    fn list_marker_kept_with_single_space() {
        assert_eq!(format("*   星号\n+   加号\n-   减号\n"),
                   "* 星号\n\n+ 加号\n\n- 减号\n");
    }

    #[test]
    fn same_marker_list_no_gap() {
        assert_eq!(format("* 一\n* 二\n* 三\n"), "* 一\n* 二\n* 三\n");
    }

    #[test]
    fn table_cjk_width_alignment() {
        let src = "|短|很长很长很长很长很长的列|\n|---|---|\n|1|22|\n|333|4|\n";
        let got = format(src);
        assert_eq!(
            got,
            "| 短  | 很长很长很长很长很长的列 |\n|-----|--------------------------|\n| 1   | 22                       |\n| 333 | 4                        |\n"
        );
    }

    #[test]
    fn fence_content_untouched() {
        let src = "```rust\nfn  main( )  {\nlet x=1;\n}\n```\n";
        assert_eq!(format(src), src);
    }

    #[test]
    fn multiple_blank_lines_collapse_between_blocks() {
        assert_eq!(format("段落一。\n\n\n\n段落二。\n"), "段落一。\n\n段落二。\n");
    }

    #[test]
    fn trailing_newline_fact_preserved() {
        assert_eq!(format("尾部无换行"), "尾部无换行");
        assert_eq!(format("有换行\n"), "有换行\n");
    }
}
