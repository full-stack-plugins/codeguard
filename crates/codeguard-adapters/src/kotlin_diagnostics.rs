use crate::{KotlinDiagnostic, KotlinParsed};
use std::collections::BTreeSet;

/// 解析固定 Kotlin/JVM 输出；只接受冻结输入的头、原行和 caret。
/// 参数为 stderr、实际冻结路径及 UTF-8 源码；返回脱敏语法/上下文定位，未知输出返回 None。
pub fn parse_kotlin_diagnostics(
    stderr: &[u8],
    input_path: &str,
    source: &str,
) -> Option<KotlinParsed> {
    let text = std::str::from_utf8(stderr).ok()?;
    let mut parsed = KotlinParsed {
        syntax: Vec::new(),
        context: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut expected_context = None;
    for row in text.lines() {
        if let Some(rest) = row
            .strip_prefix(input_path)
            .and_then(|s| s.strip_prefix(':'))
        {
            let (line, rest) = rest.split_once(':')?;
            let (column, message) = rest.split_once(": error: [")?;
            let (code, message) = message.split_once("] ")?;
            if code.is_empty()
                || code.len() > 128
                || message.is_empty()
                || !code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                return None;
            }
            let line = line.parse::<usize>().ok()?;
            let column_utf16 = column.parse::<usize>().ok()?;
            let source_line = source
                .split('\n')
                .nth(line.checked_sub(1)?)?
                .trim_end_matches('\r');
            let column_byte = utf16_column_to_byte(source_line, column_utf16)?;
            expected_context = Some(source_line);
            let key = (code.to_owned(), line, column_byte);
            if !seen.insert(key) {
                continue;
            }
            if seen.len() > 32 {
                return None;
            }
            let diagnostic = KotlinDiagnostic {
                rule_id: if code == "SYNTAX" {
                    "kotlin.syntax".into()
                } else {
                    format!("kotlin.context.{code}")
                },
                line,
                column_byte,
                column_utf16,
            };
            if code == "SYNTAX" {
                parsed.syntax.push(diagnostic);
            } else {
                parsed.context.push(diagnostic);
            }
        } else if Some(row) == expected_context
            || row.trim().is_empty()
            || (expected_context.is_some()
                && row.contains('^')
                && row.bytes().all(|b| matches!(b, b' ' | b'\t' | b'^' | b'~')))
        {
            continue;
        } else {
            return None;
        }
    }
    Some(parsed)
}

fn utf16_column_to_byte(line: &str, column: usize) -> Option<usize> {
    let target = column.checked_sub(1)?;
    let mut units = 0;
    for (byte, ch) in line.char_indices() {
        if units == target {
            return Some(byte + 1);
        }
        units += ch.len_utf16();
        if units > target {
            return None;
        }
    }
    (units == target).then_some(line.len() + 1)
}
