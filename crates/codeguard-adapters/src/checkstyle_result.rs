use crate::{CheckstyleParsed, parse_checkstyle_xml};
use std::collections::HashSet;
use std::path::Path;

/// Checkstyle 本地报告、冻结文件列表与原生退出的相互一致性，不是交付或规则覆盖证明。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckstyleResult {
    /// 本地范围/版本/退出一致；仍须独立核验输入身份、配置、产物与策略。
    pub local_coherent: bool,
    /// 原始解析观察；未完成时仅作局部证据，不能自动映射活动源码 finding。
    pub parsed: CheckstyleParsed,
    /// 稳定未完成原因。
    pub reason: Option<&'static str>,
}

/// 核对原始 XML、独立版本/完整文件列表与正常退出码。
/// 参数 exit_code 为正常退出时的值，信号/取消/超时传 None；返回本地观察，不授予白名单或门禁。
#[must_use]
pub fn evaluate_checkstyle_report(
    bytes: &[u8],
    version: &str,
    expected_files: &[String],
    exit_code: Option<i32>,
) -> CheckstyleResult {
    let parsed = parse_checkstyle_xml(bytes, version);
    let reason = if !parsed.report_valid {
        Some(parsed.reason.unwrap_or("checkstyle_report_incomplete"))
    } else if version != "10.21.4" || !cfg!(unix) {
        Some("checkstyle_exit_contract_unverified")
    } else {
        coherence_reason(&parsed, expected_files, exit_code)
    };
    CheckstyleResult {
        local_coherent: reason.is_none(),
        parsed,
        reason,
    }
}

fn coherence_reason(
    parsed: &CheckstyleParsed,
    expected_files: &[String],
    exit_code: Option<i32>,
) -> Option<&'static str> {
    let expected: HashSet<_> = expected_files.iter().map(String::as_str).collect();
    if expected.is_empty()
        || expected.len() != expected_files.len()
        || expected.iter().any(|name| !Path::new(name).is_absolute())
    {
        return Some("checkstyle_expected_scope_invalid");
    }
    let actual: HashSet<_> = parsed.files.iter().map(String::as_str).collect();
    if actual != expected {
        return Some("checkstyle_report_scope_mismatch");
    }
    // Main 将 Checker 的 error 数作为 JVM 退出值；Unix wait 只保留低八位。
    // warning/info 即使退出零也保留，256 个 error 也不能被归为空成功。
    let errors = parsed
        .diagnostics
        .iter()
        .filter(|d| d.severity == "error")
        .count();
    if exit_code != Some((errors % 256) as i32) {
        return Some("checkstyle_exit_report_conflict");
    }
    None
}
