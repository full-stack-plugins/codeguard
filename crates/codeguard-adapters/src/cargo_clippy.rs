//! Cargo Clippy 的逐行机器报告解析；非 Clippy 编译诊断不能伪装成 lint 发现。

use serde_json::Value;

/// 一条带原生规则身份的 Clippy 发现。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClippyFinding {
    /// 原生规则 ID。
    pub rule_id: String,
    /// 原生目标路径。
    pub path: String,
    /// 诊断起始行。
    pub line: u64,
    /// 诊断起始列。
    pub column: u64,
    /// 原生诊断级别。
    pub level: String,
}

/// 一轮 Cargo JSON 流的保守解析结果。
#[derive(Debug)]
pub struct ClippyParsed {
    /// 具备原生规则 ID 的发现。
    pub findings: Vec<ClippyFinding>,
    /// 是否看见唯一的、成功的终结记录。
    pub build_finished: bool,
    /// 不能用于完整判定的解析原因。
    pub issue: Option<&'static str>,
}

/// 解析 Cargo `--message-format=json`；坏行、缺结束记录及非 Clippy 错误均不可判完整。
#[must_use]
pub fn parse_cargo_clippy_json(bytes: &[u8]) -> ClippyParsed {
    let mut parsed = ClippyParsed {
        findings: Vec::new(),
        build_finished: false,
        issue: None,
    };
    let Ok(output) = std::str::from_utf8(bytes) else {
        parsed.issue = Some("native_report_non_utf8");
        return parsed;
    };
    let mut finish_count = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message): Result<Value, _> = serde_json::from_str(line) else {
            parsed.issue = Some("native_report_malformed");
            break;
        };
        match message["reason"].as_str() {
            Some("compiler-message") => {
                let Some(diagnostic) = message.get("message") else {
                    parsed.issue = Some("native_report_malformed");
                    break;
                };
                let level = diagnostic["level"].as_str().unwrap_or("");
                let rule = diagnostic["code"]["code"].as_str().unwrap_or("");
                if rule.starts_with("clippy::") && matches!(level, "warning" | "error") {
                    let location = diagnostic["spans"]
                        .as_array()
                        .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true));
                    let Some(location) = location else {
                        parsed.issue = Some("native_finding_location_missing");
                        break;
                    };
                    let (Some(path), Some(line), Some(column)) = (
                        location["file_name"].as_str(),
                        location["line_start"].as_u64(),
                        location["column_start"].as_u64(),
                    ) else {
                        parsed.issue = Some("native_finding_location_missing");
                        break;
                    };
                    parsed.findings.push(ClippyFinding {
                        rule_id: rule.into(),
                        path: path.into(),
                        line,
                        column,
                        level: level.into(),
                    });
                } else if level == "error" {
                    parsed.issue = Some("non_clippy_compilation_error");
                }
            }
            Some("build-finished") => {
                finish_count += 1;
                parsed.build_finished = message["success"] == true;
            }
            Some("compiler-artifact" | "build-script-executed" | "text-line") => {}
            _ => {
                parsed.issue = Some("native_report_unknown_record");
                break;
            }
        }
    }
    if finish_count != 1 && parsed.issue.is_none() {
        parsed.issue = Some("native_report_finish_missing_or_duplicate");
    }
    if !parsed.build_finished && parsed.issue.is_none() {
        parsed.issue = Some("native_build_failed");
    }
    parsed
}
