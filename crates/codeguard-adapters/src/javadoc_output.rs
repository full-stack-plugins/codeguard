//! JDK 21 原生 Javadoc 缺失注释诊断解析；文本格式不确定时保持未完成。

/// Javadoc 原生诊断的局部状态，不代表项目级注释检查覆盖。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavadocParseState {
    ValidDiagnostics,
    Incomplete,
}

/// 有界、脱敏的 Javadoc 诊断位置与规则身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavadocDiagnostic {
    pub rule_id: &'static str,
    pub line: u32,
    pub column: u32,
}

/// 原生 stderr 的解析结果；未知文字不能变为干净检查。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavadocParsed {
    pub state: JavadocParseState,
    pub diagnostics: Vec<JavadocDiagnostic>,
    pub reason: Option<&'static str>,
}

/// 仅接受固定英语区域设置的 JDK 21 `-Xdoclint:missing -quiet` 输出。
/// 参数为原始 stderr、隔离副本绝对路径与扫描前源码字节；不读取项目文件或执行原生工具。
#[must_use]
pub fn parse_javadoc_output(bytes: &[u8], expected_file: &str, source: &[u8]) -> JavadocParsed {
    parse_output(bytes, expected_file, source, false)
}

/// 解析 JDK21 缺注释及详细描述诊断；参数为原生输出、隔离路径和扫描源码。
/// 返回源位置绑定的局部诊断；未知文字保持未完成，不授予项目覆盖资格。
#[must_use]
pub fn parse_detailed_javadoc_output(
    bytes: &[u8],
    expected_file: &str,
    source: &[u8],
) -> JavadocParsed {
    parse_output(bytes, expected_file, source, true)
}

fn parse_output(bytes: &[u8], expected_file: &str, source: &[u8], detailed: bool) -> JavadocParsed {
    if bytes.len() > 1024 * 1024 || expected_file.is_empty() || source.len() > 16 * 1024 * 1024 {
        return incomplete("javadoc_output_invalid");
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return incomplete("javadoc_output_encoding_invalid");
    };
    if text
        .bytes()
        .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\n' | b'\r' | b'\t'))
    {
        return incomplete("javadoc_output_control_character");
    }
    let mut lines: Vec<&str> = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    if lines.is_empty() {
        return JavadocParsed {
            state: JavadocParseState::ValidDiagnostics,
            diagnostics: Vec::new(),
            reason: None,
        };
    }
    let Some(summary) = lines.pop() else {
        return incomplete("javadoc_output_invalid");
    };
    let expected_count = if let Some(value) = summary.strip_suffix(" warnings") {
        value.parse::<usize>().ok()
    } else if let Some(value) = summary.strip_suffix(" warning") {
        value.parse::<usize>().ok().filter(|count| *count == 1)
    } else {
        None
    };
    let Some(expected_count) = expected_count.filter(|count| *count <= 10_000) else {
        return incomplete("javadoc_summary_unrecognized");
    };
    if lines.len() != expected_count.saturating_mul(3) {
        return incomplete("javadoc_diagnostic_count_mismatch");
    }
    let mut diagnostics = Vec::with_capacity(expected_count);
    for chunk in lines.chunks_exact(3) {
        let Some(rest) = chunk[0]
            .strip_prefix(expected_file)
            .and_then(|rest| rest.strip_prefix(':'))
        else {
            return incomplete("javadoc_diagnostic_out_of_scope");
        };
        let Some((line_text, message)) = rest.split_once(": warning: ") else {
            return incomplete("javadoc_diagnostic_unrecognized");
        };
        let Some(line) = line_text.parse::<u32>().ok().filter(|line| *line > 0) else {
            return incomplete("javadoc_location_invalid");
        };
        if source.split(|byte| *byte == b'\n').nth((line - 1) as usize) != Some(chunk[1].as_bytes())
        {
            return incomplete("javadoc_source_line_mismatch");
        }
        let Some(rule_id) =
            rule_id(message).or_else(|| detailed.then(|| detailed_rule_id(message)).flatten())
        else {
            return incomplete("javadoc_rule_unrecognized");
        };
        let caret = chunk[2];
        let Some(prefix) = caret.strip_suffix('^') else {
            return incomplete("javadoc_location_invalid");
        };
        if !prefix.bytes().all(|byte| byte == b' ') || chunk[1].is_empty() {
            return incomplete("javadoc_location_invalid");
        }
        let Ok(column) = u32::try_from(prefix.len() + 1) else {
            return incomplete("javadoc_location_invalid");
        };
        diagnostics.push(JavadocDiagnostic {
            rule_id,
            line,
            column,
        });
    }
    JavadocParsed {
        state: JavadocParseState::ValidDiagnostics,
        diagnostics,
        reason: None,
    }
}

fn rule_id(message: &str) -> Option<&'static str> {
    match message {
        "no comment" => Some("JavadocMissingComment"),
        "use of default constructor, which does not provide a comment" => {
            Some("JavadocDefaultConstructorMissingComment")
        }
        "no @return" => Some("JavadocMissingReturn"),
        _ => {
            let (prefix, rule) = if let Some(name) = message.strip_prefix("no @param for ") {
                (name, "JavadocMissingParam")
            } else {
                (
                    message.strip_prefix("no @throws for ")?,
                    "JavadocMissingThrows",
                )
            };
            (!prefix.is_empty()
                && prefix.len() <= 128
                && prefix.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.' | b'<' | b'>')
                }))
            .then_some(rule)
        }
    }
}

fn incomplete(reason: &'static str) -> JavadocParsed {
    JavadocParsed {
        state: JavadocParseState::Incomplete,
        diagnostics: Vec::new(),
        reason: Some(reason),
    }
}

fn detailed_rule_id(message: &str) -> Option<&'static str> {
    match message {
        "empty comment" => Some("JavadocEmptyComment"),
        "no main description" => Some("JavadocMissingMainDescription"),
        "no description for @param" => Some("JavadocEmptyParamDescription"),
        "no description for @return" => Some("JavadocEmptyReturnDescription"),
        "no description for @throws" => Some("JavadocEmptyThrowsDescription"),
        _ => None,
    }
}
