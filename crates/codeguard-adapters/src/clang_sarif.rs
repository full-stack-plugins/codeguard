//! 固定 Clang stdin SARIF 读者；不使用诊断自由文本或外部文件定位。
use serde_json::{Value, json};

/// 从固定Apple Clang21的SARIF提取同一冻结源码的原生规则及UTF-8字节位置。
/// 参数为有界报告、源码与原生成功状态；未知身份/列单位/报告矛盾返回错误。
pub fn parse_clang_stdin_sarif(
    bytes: &[u8],
    source: &[u8],
    success: bool,
) -> Result<Vec<Value>, &'static str> {
    if bytes.len() > 64 * 1024 || source.len() > 1024 * 1024 {
        return Err("clang_report_invalid");
    }
    let text = std::str::from_utf8(source).map_err(|_| "clang_report_invalid")?;
    let report = crate::parse_unique_json(bytes).map_err(|_| "clang_report_invalid")?;
    let runs = report["runs"]
        .as_array()
        .filter(|rows| rows.len() == 1)
        .ok_or("clang_report_invalid")?;
    let run = &runs[0];
    let driver = &run["tool"]["driver"];
    if report["version"] != "2.1.0"
        || run["columnKind"] != "unicodeCodePoints"
        || driver["name"] != "clang"
        || driver["version"] != "Apple clang version 21.0.0 (clang-2100.3.34.2)"
        || !run["invocations"]
            .as_array()
            .is_some_and(|rows| rows.len() == 1 && rows[0]["executionSuccessful"] == success)
    {
        return Err("clang_report_invalid");
    }
    let results = run["results"]
        .as_array()
        .filter(|rows| rows.len() <= 64)
        .ok_or("clang_report_invalid")?;
    let rules = driver["rules"].as_array().ok_or("clang_report_invalid")?;
    let artifacts = run["artifacts"]
        .as_array()
        .filter(|rows| rows.len() <= 1)
        .ok_or("clang_report_invalid")?;
    if artifacts
        .iter()
        .any(|a| a["location"]["uri"] != "file://" || a["location"]["index"] != 0)
    {
        return Err("clang_report_invalid");
    }
    let mut diagnostics = Vec::new();
    let mut error_count = 0;
    for result in results {
        let id = result["ruleId"]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            .ok_or("clang_report_invalid")?;
        let index = result["ruleIndex"]
            .as_u64()
            .and_then(|i| usize::try_from(i).ok())
            .ok_or("clang_report_invalid")?;
        if rules.get(index).is_none_or(|r| r["id"] != id) {
            return Err("clang_report_invalid");
        }
        let level = result["level"]
            .as_str()
            .filter(|level| matches!(*level, "error" | "warning" | "note"))
            .ok_or("clang_report_invalid")?;
        let locations = result["locations"]
            .as_array()
            .filter(|rows| !rows.is_empty() && rows.len() <= 4)
            .ok_or("clang_report_invalid")?;
        let mut first = None;
        for location in locations {
            let physical = &location["physicalLocation"];
            if artifacts.len() != 1
                || physical["artifactLocation"]["uri"] != "file://"
                || physical["artifactLocation"]["index"] != 0
            {
                return Err("clang_report_invalid");
            }
            let region = &physical["region"];
            let line = region["startLine"]
                .as_u64()
                .filter(|n| *n > 0)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("clang_report_invalid")?;
            let column = region["startColumn"]
                .as_u64()
                .filter(|n| *n > 0)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("clang_report_invalid")?;
            let source_line = text
                .split('\n')
                .nth(line - 1)
                .ok_or("clang_report_invalid")?;
            let offset = if column == source_line.chars().count() + 1 {
                source_line.len()
            } else {
                source_line
                    .char_indices()
                    .nth(column - 1)
                    .map(|(offset, _)| offset)
                    .ok_or("clang_report_invalid")?
            };
            if first.is_none() {
                first = Some(
                    json!({"rule_id":format!("clang.{id}"),"line":line,"column_byte":offset+1,"level":level}),
                );
            }
        }
        if level == "error" {
            error_count += 1;
        }
        if level != "note" {
            diagnostics.push(first.ok_or("clang_report_invalid")?);
        }
    }
    if success != (error_count == 0) {
        return Err("clang_report_invalid");
    }
    Ok(diagnostics)
}
