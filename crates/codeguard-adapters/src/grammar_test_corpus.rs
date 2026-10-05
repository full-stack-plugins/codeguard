use crate::GrammarTestSample;

/// 读取标准三行标题的 Tree-sitter corpus，保留源码字节并核对预期树。
/// 输入至多 16 MiB UTF-8 文本；返回全部样本或具体拒绝原因。
/// 不支持的多行标题/指令不被静默忽略，grammar 自带标签不具原生裁定权威。
pub fn parse_grammar_test_corpus(text: &str) -> Result<Vec<GrammarTestSample>, String> {
    if text.is_empty() || text.len() > 16 * 1024 * 1024 {
        return Err("grammar_corpus_size_invalid".into());
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut cursor = 0;
    let mut cases = Vec::new();
    while cursor < lines.len() {
        while cursor < lines.len() && lines[cursor].trim().is_empty() {
            cursor += 1;
        }
        if cursor == lines.len() {
            break;
        }
        if !header(&lines, cursor) {
            return Err("grammar_corpus_header_or_directive_unsupported".into());
        }
        let name = line_body(lines[cursor + 1]).to_owned();
        if name.is_empty() || name.starts_with(':') || name.chars().any(char::is_control) {
            return Err("grammar_corpus_title_invalid".into());
        }
        cursor += 3;
        let start = cursor;
        while cursor < lines.len() && !separator(lines[cursor], b'-', 3) {
            if header(&lines, cursor) {
                return Err("grammar_corpus_expectation_missing".into());
            }
            cursor += 1;
        }
        if cursor == lines.len() {
            return Err("grammar_corpus_expectation_missing".into());
        }
        let source = lines[start..cursor].concat();
        if source.len() > 1024 * 1024 {
            return Err("grammar_corpus_source_too_large".into());
        }
        cursor += 1;
        let start = cursor;
        while cursor < lines.len() && !header(&lines, cursor) {
            cursor += 1;
        }
        let expected_error = expectation_error(&lines[start..cursor].concat())?;
        cases.push(GrammarTestSample {
            name,
            source,
            expected_error,
        });
        if cases.len() > 4096 {
            return Err("grammar_corpus_case_budget_exceeded".into());
        }
    }
    if cases.is_empty() {
        return Err("grammar_corpus_empty".into());
    }
    Ok(cases)
}

fn header(lines: &[&str], cursor: usize) -> bool {
    cursor + 2 < lines.len()
        && separator(lines[cursor], b'=', 4)
        && separator(lines[cursor + 2], b'=', 4)
}

fn line_body(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn separator(line: &str, byte: u8, minimum: usize) -> bool {
    let body = line_body(line);
    body.len() >= minimum && body.bytes().all(|b| b == byte)
}

fn expectation_error(text: &str) -> Result<bool, String> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut roots = 0;
    let mut cursor = 0;
    let mut error = false;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => {
                depth += 1;
                roots += 1;
                if depth > 4096 {
                    return Err("grammar_corpus_expectation_depth_exceeded".into());
                }
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                let start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && !matches!(bytes[cursor], b'(' | b')' | b'"')
                {
                    cursor += 1;
                }
                if start == cursor {
                    return Err("grammar_corpus_expectation_node_missing".into());
                }
                error |= matches!(&bytes[start..cursor], b"ERROR" | b"MISSING");
            }
            b')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("grammar_corpus_expectation_unbalanced")?;
                cursor += 1;
            }
            b'"' => {
                if depth == 0 {
                    return Err("grammar_corpus_expectation_invalid".into());
                }
                cursor += 1;
                let mut closed = false;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'\\' {
                        cursor += 2;
                        continue;
                    }
                    if bytes[cursor] == b'"' {
                        cursor += 1;
                        closed = true;
                        break;
                    }
                    cursor += 1;
                }
                if !closed {
                    return Err("grammar_corpus_expectation_unclosed_string".into());
                }
            }
            b if b.is_ascii_whitespace() => cursor += 1,
            _ if depth > 0 => cursor += 1,
            _ => return Err("grammar_corpus_expectation_invalid".into()),
        }
    }
    if depth != 0 || roots == 0 {
        return Err("grammar_corpus_expectation_unbalanced".into());
    }
    Ok(error)
}
