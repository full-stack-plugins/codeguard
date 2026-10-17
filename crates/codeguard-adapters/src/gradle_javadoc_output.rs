//! Gradle转发的英语JDK21缺注释warning块解析，未知输出不猜测源码违规。
use crate::{GradleJavadocDiagnostic, JavadocParseState, parse_javadoc_output};
use std::collections::BTreeMap;
/// 按每次Javadoc的warning汇总解析多个连续块，精确核对已选源码字节。
/// 参数为原生stderr和绝对源码映射；返回局部位置，空输出不证明任务执行或规则完整。
pub fn parse_gradle_javadoc_output(
    bytes: &[u8],
    sources: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<GradleJavadocDiagnostic>, &'static str> {
    if bytes.len() > 1024 * 1024 || sources.is_empty() || sources.len() > 128 {
        return Err("gradle_javadoc_output_budget_invalid");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "gradle_javadoc_output_encoding_invalid")?;
    let mut block: Vec<&str> = Vec::new();
    let mut result = Vec::new();
    for line in text.lines().map(|s| s.trim_end_matches('\r')) {
        if line.is_empty() && block.is_empty() {
            continue;
        }
        let count = line
            .strip_suffix(" warnings")
            .and_then(|n| n.parse::<usize>().ok())
            .or_else(|| (line == "1 warning").then_some(1));
        if let Some(count) = count {
            if count == 0
                || count > 10_000
                || block.len() != count * 3
                || result.len() + count > 10_000
            {
                return Err("gradle_javadoc_diagnostic_count_mismatch");
            }
            for chunk in block.chunks_exact(3) {
                let matches = sources
                    .iter()
                    .filter(|(path, _)| {
                        chunk[0]
                            .strip_prefix(path.as_str())
                            .is_some_and(|s| s.starts_with(':'))
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err("gradle_javadoc_source_unresolved");
                }
                let (path, source) = matches[0];
                // JDK21实际输出已验证的描述缺失；复用位置核验，保留各规则身份，旧独立Javadoc协议不扩大。
                let empty_description = [
                    ("empty comment", "JavadocEmptyComment"),
                    ("no main description", "JavadocMissingMainDescription"),
                    (
                        "no description for @throws",
                        "JavadocEmptyThrowsDescription",
                    ),
                    ("no description for @param", "JavadocEmptyParamDescription"),
                    (
                        "no description for @return",
                        "JavadocEmptyReturnDescription",
                    ),
                ]
                .iter()
                .find(|(message, _)| chunk[0].ends_with(&format!(": warning: {message}")))
                .copied();
                let header = if let Some((message, _)) = empty_description {
                    format!(
                        "{}no comment",
                        chunk[0].strip_suffix(message).expect("已匹配后缀")
                    )
                } else {
                    chunk[0].to_owned()
                };
                let one = format!("{header}\n{}\n{}\n1 warning\n", chunk[1], chunk[2]);
                let parsed = parse_javadoc_output(one.as_bytes(), path, source);
                if parsed.state != JavadocParseState::ValidDiagnostics
                    || parsed.diagnostics.len() != 1
                {
                    return Err("gradle_javadoc_diagnostic_invalid");
                }
                let d = &parsed.diagnostics[0];
                result.push(GradleJavadocDiagnostic {
                    path: path.clone(),
                    rule_id: empty_description.map_or(d.rule_id, |(_, rule)| rule),
                    line: d.line,
                    column: d.column,
                });
            }
            block.clear();
        } else {
            block.push(line);
        }
    }
    if !block.is_empty() {
        return Err("gradle_javadoc_output_incomplete");
    }
    Ok(result)
}
