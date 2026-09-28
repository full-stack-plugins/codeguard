//! Cargo check 构建协议解析；不复用 Clippy 或文档规则过滤器。

use crate::cargo_build_diagnostic::CargoBuildDiagnostic;
use crate::cargo_build_parsed::CargoBuildParsed;
use crate::strict_json::parse_unique_json;
use serde_json::Value;

/// 解析原生 stdout 与进程退出码，返回编译观察及未完成原因；不授予项目覆盖或交付通过。
#[must_use]
pub fn parse_cargo_build_json(bytes: &[u8], exit_code: i32) -> CargoBuildParsed {
    let mut parsed = CargoBuildParsed {
        diagnostics: Vec::new(),
        build_success: None,
        issue: None,
    };
    if bytes.len() > 16 * 1024 * 1024 {
        parsed.issue = Some("native_report_limit_exceeded");
        return parsed;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        parsed.issue = Some("native_report_non_utf8");
        return parsed;
    };
    for (index, line) in text.lines().enumerate() {
        if index >= 100_000 || line.len() > 1024 * 1024 {
            parsed.issue = Some("native_report_limit_exceeded");
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if parsed.build_success.is_some() {
            parsed.issue = Some("native_report_event_after_finish");
            break;
        }
        let Ok(row) = parse_unique_json(line.as_bytes()) else {
            parsed.issue = Some("native_report_malformed");
            break;
        };
        match row["reason"].as_str() {
            Some("compiler-message") => match diagnostic(&row) {
                Ok(Some(value)) => parsed.diagnostics.push(value),
                Ok(None) => {}
                Err(issue) => {
                    parsed.issue = Some(issue);
                    break;
                }
            },
            Some("build-finished") => {
                let Some(success) = row["success"].as_bool() else {
                    parsed.issue = Some("native_report_malformed");
                    break;
                };
                parsed.build_success = Some(success);
            }
            Some("compiler-artifact" | "build-script-executed") => {}
            _ => {
                parsed.issue = Some("native_report_unknown_record");
                break;
            }
        }
    }
    // 裸退出、缺结束与互相矛盾的原生状态不能签发完整观察。
    if parsed.issue.is_none() {
        parsed.issue = match parsed.build_success {
            None => Some("native_report_finish_missing"),
            Some(true) if !parsed.diagnostics.is_empty() => Some("native_build_success_with_error"),
            Some(true) if exit_code != 0 => Some("native_build_exit_mismatch"),
            Some(false) if exit_code != 101 => Some("native_build_exit_mismatch"),
            Some(false) if parsed.diagnostics.is_empty() => {
                Some("native_build_failure_unattributed")
            }
            _ => None,
        };
    }
    parsed
}

fn diagnostic(row: &Value) -> Result<Option<CargoBuildDiagnostic>, &'static str> {
    let message = row
        .get("message")
        .filter(|value| value.is_object())
        .ok_or("native_report_malformed")?;
    match message["level"].as_str() {
        Some("warning" | "note" | "help" | "failure-note") => return Ok(None),
        Some("error") => {}
        _ => return Err("native_report_unknown_diagnostic_level"),
    }
    let code = identity(&message["code"]["code"]).ok_or("native_compiler_error_unattributed")?;
    let code_bytes = code.as_bytes();
    if code_bytes.len() != 5
        || code_bytes[0] != b'E'
        || !code_bytes[1..].iter().all(u8::is_ascii_digit)
    {
        return Err("native_compiler_error_code_unsupported");
    }
    let spans = message["spans"]
        .as_array()
        .filter(|spans| spans.len() <= 10_000)
        .ok_or("native_compiler_location_ambiguous")?;
    let mut primary = spans.iter().filter(|span| span["is_primary"] == true);
    let span = primary.next().ok_or("native_compiler_location_ambiguous")?;
    if primary.next().is_some() {
        return Err("native_compiler_location_ambiguous");
    }
    let start = span["byte_start"]
        .as_u64()
        .ok_or("native_compiler_range_invalid")?;
    let end = span["byte_end"]
        .as_u64()
        .filter(|end| *end >= start)
        .ok_or("native_compiler_range_invalid")?;
    let kinds = row["target"]["kind"]
        .as_array()
        .filter(|kinds| !kinds.is_empty() && kinds.len() <= 16)
        .ok_or("native_target_identity_missing")?;
    let target_kinds = kinds
        .iter()
        .map(|kind| identity(kind).ok_or("native_target_identity_missing"))
        .collect::<Result<Vec<_>, _>>()?;
    let result = CargoBuildDiagnostic {
        code,
        path: identity(&span["file_name"]).ok_or("native_compiler_location_missing")?,
        line: span["line_start"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("native_compiler_location_missing")?,
        column: span["column_start"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("native_compiler_location_missing")?,
        byte_start: start,
        byte_end: end,
        package_id: identity(&row["package_id"]).ok_or("native_target_identity_missing")?,
        manifest_path: identity(&row["manifest_path"]).ok_or("native_target_identity_missing")?,
        target_source: identity(&row["target"]["src_path"])
            .ok_or("native_target_identity_missing")?,
        target_kinds,
    };
    Ok(Some(result))
}

fn identity(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| {
            !text.is_empty() && text.len() <= 8192 && !text.chars().any(char::is_control)
        })
        .map(str::to_owned)
}
