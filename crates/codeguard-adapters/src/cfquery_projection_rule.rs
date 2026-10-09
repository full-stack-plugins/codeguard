use sha2::{Digest, Sha256};
const RULE: &[u8] = include_bytes!("../../../rulepacks/cfquery/distinct_projection.json");

/// 返回独立 CFQuery 结构候选规则的固定字节摘要；不授予数据库方言或规则资格。
#[must_use]
pub fn cfquery_projection_rule_sha256() -> String {
    format!("{:x}", Sha256::digest(RULE))
}

/// 核对关键词锚点只包含 SELECT、DISTINCT、FROM 及 SQL 空白/注释；拒绝伪造坐标。
#[must_use]
pub fn cfquery_projection_span_valid(source: &[u8]) -> bool {
    let mut cursor = 0;
    for (index, word) in [b"SELECT".as_slice(), b"DISTINCT", b"FROM"]
        .iter()
        .enumerate()
    {
        if index > 0 {
            let before = cursor;
            loop {
                if source.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                    cursor += 1;
                    continue;
                }
                if source.get(cursor..).is_some_and(|b| b.starts_with(b"--")) {
                    cursor += 2;
                    while source.get(cursor).is_some_and(|b| *b != b'\n') {
                        cursor += 1;
                    }
                    continue;
                }
                if source.get(cursor..).is_some_and(|b| b.starts_with(b"/*")) {
                    cursor += 2;
                    let mut depth = 1;
                    while depth > 0 {
                        let Some(tail) = source.get(cursor..).filter(|b| !b.is_empty()) else {
                            return false;
                        };
                        if tail.starts_with(b"/*") {
                            depth += 1;
                            cursor += 2;
                        } else if tail.starts_with(b"*/") {
                            depth -= 1;
                            cursor += 2;
                        } else {
                            cursor += 1;
                        }
                        if depth > 64 {
                            return false;
                        }
                    }
                    continue;
                }
                break;
            }
            if before == cursor {
                return false;
            }
        }
        if !source
            .get(cursor..cursor + word.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(word))
        {
            return false;
        }
        cursor += word.len();
    }
    cursor == source.len()
}

#[cfg(test)]
mod tests {
    use super::cfquery_projection_span_valid;
    #[test]
    fn span_does_not_accept_injected_literals_interpolation_or_unclosed_comments() {
        for valid in [
            "SELECT DISTINCT FROM",
            "select\ndistinct\tfrom",
            "SELECT DISTINCT /* nested /* x */ y */ FROM",
            "SELECT DISTINCT -- x\nFROM",
        ] {
            assert!(cfquery_projection_span_valid(valid.as_bytes()), "{valid}");
        }
        for invalid in [
            "SELECTFROM",
            "SELECT DISTINCT 'x' FROM",
            "SELECT DISTINCT #x# FROM",
            "SELECT DISTINCT /* unterminated FROM",
            "SELECT DISTINCT FROM users",
            "SELECT DISTINCT \"from\" FROM",
            "SELECT DISTINCT\nFROManything",
        ] {
            assert!(
                !cfquery_projection_span_valid(invalid.as_bytes()),
                "{invalid}"
            );
        }
    }
}
