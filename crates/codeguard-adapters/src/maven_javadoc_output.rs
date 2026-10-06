//! Maven Javadoc 原生输出的保守解析；构建成功不能吞掉缺注释警告。

use std::collections::BTreeMap;

use crate::javadoc_output::{JavadocParseState, parse_detailed_javadoc_output, parse_javadoc_output};

/// Maven Javadoc 输出是否可归属到已知源码和规则。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MavenJavadocParseState {
    ValidDiagnostics,
    /// 日志无诊断，但无法仅凭日志证明本轮重新执行了 Javadoc。
    CleanLogUnverified,
    Incomplete,
}

/// 从 Maven 原生 Javadoc 块还原的局部诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MavenJavadocDiagnostic {
    pub path: String,
    pub rule_id: &'static str,
    pub line: u32,
    pub column: u32,
}

/// 解析结果；无法证明原生输出归属时不得保留部分诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MavenJavadocParsed {
    pub state: MavenJavadocParseState,
    pub diagnostics: Vec<MavenJavadocDiagnostic>,
    pub reason: Option<&'static str>,
}

/// 将单一 Maven Javadoc goal 的原生日志映射到已知源码字节。
/// 参数包含合并后的有界日志、原生退出码、固定插件版本和本次源码快照。
#[must_use]
pub fn parse_maven_javadoc_output(
    bytes: &[u8],
    native_exit: i32,
    plugin_version: &str,
    sources: &BTreeMap<String, Vec<u8>>,
) -> MavenJavadocParsed {
    parse_output(bytes, native_exit, plugin_version, sources, false)
}

/// 解析Maven原POM的JDK21详细描述诊断；参数为有界日志、退出码、插件版本与源码快照。
/// 返回源字节绑定的局部诊断；离线插件缺失属于环境阻塞，未知输出保持未完成。
#[must_use]
pub fn parse_detailed_maven_javadoc_output(
    bytes: &[u8],
    native_exit: i32,
    plugin_version: &str,
    sources: &BTreeMap<String, Vec<u8>>,
) -> MavenJavadocParsed {
    parse_output(bytes, native_exit, plugin_version, sources, true)
}

fn parse_output(
    bytes: &[u8],
    native_exit: i32,
    plugin_version: &str,
    sources: &BTreeMap<String, Vec<u8>>,
    detailed: bool,
) -> MavenJavadocParsed {
    if bytes.len() > 2 * 1024 * 1024
        || sources.is_empty()
        || plugin_version.is_empty()
        || plugin_version.len() > 32
        || !plugin_version
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return incomplete("javadoc_log_or_identity_invalid");
    }
    let Ok(log) = std::str::from_utf8(bytes) else {
        return incomplete("javadoc_log_encoding_invalid");
    };
    if log
        .bytes()
        .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\n' | b'\r' | b'\t'))
    {
        return incomplete("javadoc_log_control_character");
    }
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    // 仅识别已实测的原插件离线缺失消息；其它解析/构建错误不猜测根因。
    let plugin_failure = format!(
        "[ERROR] Plugin org.apache.maven.plugins:maven-javadoc-plugin:{plugin_version} or one of its dependencies could not be resolved:"
    );
    let offline_artifact = format!(
        " in offline mode and the artifact org.apache.maven.plugins:maven-javadoc-plugin:jar:{plugin_version} has not been downloaded from it before."
    );
    if detailed
        && native_exit == 1
        && lines
            .iter()
            .filter(|line| **line == "[INFO] BUILD FAILURE")
            .count()
            == 1
        && !lines.contains(&"[INFO] BUILD SUCCESS")
        && lines.contains(&plugin_failure.as_str())
        && lines.iter().any(|line| {
            line.starts_with("[ERROR] \tCannot access ") && line.ends_with(&offline_artifact)
        })
    {
        return incomplete("maven_javadoc_offline_plugin_unavailable");
    }
    let goal_prefix = format!("[INFO] --- javadoc:{plugin_version}:javadoc ");
    let goal_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with(&goal_prefix).then_some(index))
        .collect();
    if goal_positions.len() != 1 {
        return incomplete("javadoc_goal_unverified");
    }
    let success = lines
        .iter()
        .filter(|line| **line == "[INFO] BUILD SUCCESS")
        .count();
    let failure = lines
        .iter()
        .filter(|line| **line == "[INFO] BUILD FAILURE")
        .count();
    let warning_failure_prefix = format!(
        "[ERROR] Failed to execute goal org.apache.maven.plugins:maven-javadoc-plugin:{plugin_version}:javadoc "
    );
    let warning_failure = lines
        .iter()
        .filter(|line| {
            line.starts_with(&warning_failure_prefix)
                && line.contains("Project contains Javadoc Warnings")
        })
        .count();
    let unexpected_errors = lines.iter().any(|line| {
        line.starts_with("[ERROR]")
            && !line.starts_with(&warning_failure_prefix)
            && !matches!(
                line.trim_end(),
                "[ERROR]"
                    | "[ERROR] To see the full stack trace of the errors, re-run Maven with the -e switch."
                    | "[ERROR] Re-run Maven using the -X switch to enable full debug logging."
                    | "[ERROR] For more information about the errors and possible solutions, please read the following articles:"
                    | "[ERROR] [Help 1] http://cwiki.apache.org/confluence/display/MAVEN/MojoExecutionException"
            )
    });
    if unexpected_errors
        || !matches!(
            (native_exit, success, failure, warning_failure),
            (0, 1, 0, 0) | (1, 0, 1, 1)
        )
    {
        return incomplete("javadoc_native_outcome_unrecognized");
    }
    let headers: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            (index > goal_positions[0] && *line == "[WARNING] Javadoc Warnings").then_some(index)
        })
        .collect();
    if headers.is_empty() {
        // Maven 在复用旧 Javadoc 产物时也可能打印相同的成功日志。
        // 只识别已观察到的无关编码提示；其它警告不得被解释为干净结果。
        let known_encoding_warning = "[WARNING] Source files encoding has not been set, using platform encoding UTF-8, i.e. build is platform dependent!";
        if native_exit == 0
            && lines
                .iter()
                .all(|line| !line.starts_with("[WARNING]") || *line == known_encoding_warning)
        {
            return MavenJavadocParsed {
                state: MavenJavadocParseState::CleanLogUnverified,
                diagnostics: Vec::new(),
                reason: Some("javadoc_fresh_execution_unverified"),
            };
        }
        return incomplete("javadoc_diagnostic_block_missing");
    }
    if headers.len() != 1 {
        return incomplete("javadoc_diagnostic_block_invalid");
    }
    let header = headers[0];
    let summary = lines
        .iter()
        .enumerate()
        .skip(header + 1)
        .find_map(|(index, line)| warning_count(line).map(|count| (index, count)));
    let Some((summary_index, expected_count)) = summary else {
        return incomplete("javadoc_warning_summary_missing");
    };
    if expected_count == 0
        || expected_count > 10_000
        || summary_index - header - 1 != expected_count.saturating_mul(3)
    {
        return incomplete("javadoc_warning_count_mismatch");
    }
    if lines[summary_index + 1..]
        .iter()
        .any(|line| line.starts_with("[WARNING] ") && line.contains(": warning: "))
    {
        return incomplete("javadoc_warning_outside_block");
    }
    let mut diagnostics = Vec::with_capacity(expected_count);
    for chunk in lines[header + 1..summary_index].chunks_exact(3) {
        let Some(first) = chunk[0].strip_prefix("[WARNING] ") else {
            return incomplete("javadoc_warning_format_invalid");
        };
        let matches: Vec<_> = sources
            .iter()
            .filter(|(path, _)| {
                first
                    .strip_prefix(path.as_str())
                    .is_some_and(|rest| rest.starts_with(':'))
            })
            .collect();
        let [(path, source)] = matches.as_slice() else {
            return incomplete("javadoc_warning_source_unverified");
        };
        let (Some(source_line), Some(caret)) = (
            chunk[1].strip_prefix("[WARNING] "),
            chunk[2].strip_prefix("[WARNING] "),
        ) else {
            return incomplete("javadoc_warning_format_invalid");
        };
        let raw = format!("{first}\n{source_line}\n{caret}\n1 warning\n");
        let parsed = if detailed {
            parse_detailed_javadoc_output(raw.as_bytes(), path, source)
        } else {
            parse_javadoc_output(raw.as_bytes(), path, source)
        };
        if parsed.state != JavadocParseState::ValidDiagnostics || parsed.diagnostics.len() != 1 {
            return incomplete("javadoc_warning_rule_or_location_unverified");
        }
        let diagnostic = &parsed.diagnostics[0];
        diagnostics.push(MavenJavadocDiagnostic {
            path: (*path).clone(),
            rule_id: diagnostic.rule_id,
            line: diagnostic.line,
            column: diagnostic.column,
        });
    }
    MavenJavadocParsed {
        state: MavenJavadocParseState::ValidDiagnostics,
        diagnostics,
        reason: None,
    }
}

fn warning_count(line: &str) -> Option<usize> {
    let value = line.strip_prefix("[WARNING] ")?;
    if let Some(count) = value.strip_suffix(" warnings") {
        count.parse().ok()
    } else {
        value
            .strip_suffix(" warning")
            .and_then(|count| count.parse::<usize>().ok())
            .filter(|count| *count == 1)
    }
}

fn incomplete(reason: &'static str) -> MavenJavadocParsed {
    MavenJavadocParsed {
        state: MavenJavadocParseState::Incomplete,
        diagnostics: Vec::new(),
        reason: Some(reason),
    }
}
