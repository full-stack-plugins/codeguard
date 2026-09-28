//! Cargo/rustdoc 机器流解析；不以 Clippy 规则或自由文本猜文档诊断。

use crate::rustdoc_finding::RustdocFinding;
use crate::rustdoc_parsed::RustdocParsed;
use crate::strict_json::parse_unique_json;
use serde_json::Value;

/// 解析有界原生 JSON 行流。输入为原生 stdout，返回观察及未完成原因，不授予覆盖权威。
#[must_use]
pub fn parse_cargo_rustdoc_json(bytes: &[u8]) -> RustdocParsed {
    let mut parsed = RustdocParsed {
        findings: Vec::new(),
        build_finished: false,
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
    let mut finished = false;
    for (index, line) in text.lines().enumerate() {
        if index >= 100_000 || line.len() > 1024 * 1024 {
            parsed.issue = Some("native_report_limit_exceeded");
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if finished {
            parsed.issue = Some("native_report_event_after_finish");
            break;
        }
        let Ok(row) = parse_unique_json(line.as_bytes()) else {
            parsed.issue = Some("native_report_malformed");
            break;
        };
        match row["reason"].as_str() {
            Some("compiler-message") => match parse_diagnostic(&row) {
                Ok(Some(finding)) => parsed.findings.push(finding),
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
                finished = true;
                parsed.build_finished = success;
                if !success {
                    parsed.issue = Some("native_build_failed");
                }
            }
            Some("compiler-artifact" | "build-script-executed") => {}
            _ => {
                parsed.issue = Some("native_report_unknown_record");
                break;
            }
        }
    }
    if !finished && parsed.issue.is_none() {
        parsed.issue = Some("native_report_finish_missing");
    }
    parsed
}

fn parse_diagnostic(row: &Value) -> Result<Option<RustdocFinding>, &'static str> {
    let diagnostic = row
        .get("message")
        .filter(|item| item.is_object())
        .ok_or("native_report_malformed")?;
    let level = diagnostic["level"]
        .as_str()
        .ok_or("native_report_malformed")?;
    if matches!(level, "note" | "help") {
        return Ok(None);
    }
    if !matches!(level, "warning" | "error") {
        return Err("native_report_unknown_diagnostic_level");
    }
    let rule = diagnostic["code"]["code"].as_str().unwrap_or("");
    if !matches!(rule, "missing_docs" | "rustdoc::broken_intra_doc_links") {
        return Err(if level == "error" {
            "non_rustdoc_compilation_error"
        } else {
            "unsupported_rustdoc_warning"
        });
    }
    let spans = diagnostic["spans"]
        .as_array()
        .ok_or("native_finding_location_ambiguous")?;
    let mut primary = spans.iter().filter(|span| span["is_primary"] == true);
    let span = primary.next().ok_or("native_finding_location_ambiguous")?;
    if primary.next().is_some() {
        return Err("native_finding_location_ambiguous");
    }
    let path = identity_string(&span["file_name"]).ok_or("native_finding_location_missing")?;
    let line = span["line_start"]
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or("native_finding_location_missing")?;
    let column = span["column_start"]
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or("native_finding_location_missing")?;
    let package_id = identity_string(&row["package_id"]).ok_or("native_target_identity_missing")?;
    let byte_start = span["byte_start"]
        .as_u64()
        .ok_or("native_finding_range_invalid")?;
    let byte_end = span["byte_end"]
        .as_u64()
        .filter(|end| *end >= byte_start)
        .ok_or("native_finding_range_invalid")?;
    let manifest_path =
        identity_string(&row["manifest_path"]).ok_or("native_target_identity_missing")?;
    let target_source =
        identity_string(&row["target"]["src_path"]).ok_or("native_target_identity_missing")?;
    let kinds = row["target"]["kind"]
        .as_array()
        .filter(|kinds| !kinds.is_empty() && kinds.len() <= 16)
        .ok_or("native_target_identity_missing")?;
    let target_kinds = kinds
        .iter()
        .map(|kind| {
            identity_string(kind)
                .map(str::to_owned)
                .ok_or("native_target_identity_missing")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(RustdocFinding {
        rule_id: rule.into(),
        path: path.into(),
        line,
        column,
        byte_start,
        byte_end,
        level: level.into(),
        package_id: package_id.into(),
        manifest_path: manifest_path.into(),
        target_source: target_source.into(),
        target_kinds,
    }))
}

fn identity_string(value: &Value) -> Option<&str> {
    value.as_str().filter(|text| {
        !text.is_empty() && text.len() <= 8192 && !text.chars().any(char::is_control)
    })
}
