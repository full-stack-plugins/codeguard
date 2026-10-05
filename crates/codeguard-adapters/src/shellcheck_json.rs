//! ShellCheck 0.11.0 json1 读者；列按原生字符计数，不复用旧 json 的 tab-stop=8。
use crate::{
    parse_unique_json, shellcheck_diagnostic::ShellCheckDiagnostic,
    shellcheck_parsed::ShellCheckParsed,
};
use serde_json::Value;
use std::collections::BTreeSet;
/// 解析原生 json1 和退出码；参数绑定冻结 UTF8源码，返回有效部分证据及环境阻塞。
pub fn parse_shellcheck_json1(bytes: &[u8], source: &[u8], exit: i32) -> ShellCheckParsed {
    let mut out = ShellCheckParsed {
        report_valid: false,
        local_scan_complete: false,
        diagnostics: Vec::new(),
        environment_codes: Vec::new(),
        reason: Some("shellcheck_report_invalid"),
    };
    if bytes.len() > 128 * 1024 || source.len() > 1024 * 1024 {
        out.reason = Some("shellcheck_report_budget_exceeded");
        return out;
    }
    let Ok(text) = std::str::from_utf8(source) else {
        return out;
    };
    let Ok(root) = parse_unique_json(bytes) else {
        return out;
    };
    if !keys(&root, &["comments"]) {
        return out;
    }
    let Some(rows) = root["comments"].as_array().filter(|rows| rows.len() <= 128) else {
        return out;
    };
    let lines: Vec<&str> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let mut valid = true;
    let mut environment = BTreeSet::new();
    for row in rows {
        let Some(diag) = diagnostic(row, &lines) else {
            valid = false;
            continue;
        };
        if matches!(
            diag.rule_id.as_str(),
            "SC1071" | "SC1090" | "SC1091" | "SC1092" | "SC1134" | "SC1144" | "SC1145"
        ) {
            environment.insert(diag.rule_id);
        } else {
            out.diagnostics.push(diag);
        }
    }
    out.diagnostics.sort();
    out.diagnostics.dedup();
    out.environment_codes = environment.into_iter().collect();
    out.report_valid = valid;
    let exit_consistent = exit == if rows.is_empty() { 0 } else { 1 };
    out.local_scan_complete = valid && exit_consistent && out.environment_codes.is_empty();
    out.reason = if !valid {
        Some("shellcheck_report_incomplete")
    } else if !exit_consistent {
        Some("shellcheck_exit_report_mismatch")
    } else if !out.environment_codes.is_empty() {
        Some("shellcheck_environment_incomplete")
    } else {
        None
    };
    out
}
fn keys(value: &Value, names: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == names.len() && names.iter().all(|k| o.contains_key(*k)))
}
fn diagnostic(row: &Value, lines: &[&str]) -> Option<ShellCheckDiagnostic> {
    if !keys(
        row,
        &[
            "file",
            "line",
            "endLine",
            "column",
            "endColumn",
            "level",
            "code",
            "message",
            "fix",
        ],
    ) || row["file"] != "-"
    {
        return None;
    }
    let number = |key: &str| {
        row[key]
            .as_u64()
            .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            .map(|n| n as u32)
    };
    let line = number("line")?;
    let column = number("column")?;
    let end_line = number("endLine")?;
    let end_column = number("endColumn")?;
    if (end_line, end_column) < (line, column)
        || !point(lines, line, column)
        || !point(lines, end_line, end_column)
    {
        return None;
    }
    let severity = row["level"]
        .as_str()
        .filter(|s| matches!(*s, "error" | "warning" | "info" | "style"))?;
    let code = row["code"].as_u64().filter(|c| (1000..=9999).contains(c))?;
    row["message"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 16 * 1024)?;
    if !valid_fix(&row["fix"]) {
        return None;
    }
    Some(ShellCheckDiagnostic {
        rule_id: format!("SC{code}"),
        severity: severity.into(),
        line,
        column,
        end_line,
        end_column,
    })
}
fn point(lines: &[&str], line: u32, column: u32) -> bool {
    lines
        .get(line as usize - 1)
        .is_some_and(|text| column as usize <= text.chars().count() + 1)
}
fn valid_fix(fix: &Value) -> bool {
    if fix.is_null() {
        return true;
    }
    keys(fix, &["replacements"])
        && fix["replacements"].as_array().is_some_and(|rows| {
            rows.len() <= 32
                && rows.iter().all(|r| {
                    keys(
                        r,
                        &[
                            "column",
                            "endColumn",
                            "endLine",
                            "insertionPoint",
                            "line",
                            "precedence",
                            "replacement",
                        ],
                    ) && ["column", "endColumn", "endLine", "line"]
                        .iter()
                        .all(|key| {
                            r[key]
                                .as_u64()
                                .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
                        })
                        && r["precedence"]
                            .as_i64()
                            .is_some_and(|n| i32::try_from(n).is_ok())
                        && matches!(
                            r["insertionPoint"].as_str(),
                            Some("afterEnd" | "beforeStart")
                        )
                        && r["replacement"].as_str().is_some_and(|s| s.len() <= 4096)
                })
        })
}
