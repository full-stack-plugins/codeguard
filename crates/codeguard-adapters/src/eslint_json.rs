//! ESLint 10 JSON 报告的有界局部观察；不执行配置、安装工具或签发门禁。
use crate::eslint_file_report::EslintFileReport;
use crate::eslint_message_report::EslintMessageReport;
use crate::{EslintDiagnostic, EslintParsed};
use std::collections::BTreeSet;
use std::path::{Component, Path};

/// 核对原生 JSON、具体版本、冻结文件集合及普通退出/max-warnings 参数。
/// 返回局部解析；解析器与抑制诊断保留未完成，未知输入不能成为干净范围。
#[must_use]
pub fn parse_eslint_json(
    bytes: &[u8],
    expected_version: &str,
    observed_version: &str,
    expected_files: &[String],
    exit_code: Option<i32>,
    max_warnings: Option<u64>,
) -> EslintParsed {
    parse_inner(
        bytes,
        expected_version,
        observed_version,
        expected_files,
        exit_code,
        max_warnings,
    )
    .unwrap_or_else(|reason| EslintParsed {
        local_coherent: false,
        findings: vec![],
        suppressed_count: 0,
        reason: Some(reason),
    })
}

fn parse_inner(
    bytes: &[u8],
    expected_version: &str,
    observed_version: &str,
    expected_files: &[String],
    exit_code: Option<i32>,
    max_warnings: Option<u64>,
) -> Result<EslintParsed, &'static str> {
    if !eslint_report_version_matches(expected_version, observed_version) {
        return Err("eslint_version_unverified");
    }
    if !matches!(exit_code, Some(0 | 1)) {
        return Err("eslint_execution_incomplete");
    }
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        return Err("eslint_report_size_invalid");
    }
    let expected: BTreeSet<_> = expected_files.iter().map(String::as_str).collect();
    if expected.is_empty()
        || expected.len() != expected_files.len()
        || expected.len() > 10_000
        || expected.iter().any(|p| !valid_path(p))
    {
        return Err("eslint_expected_scope_invalid");
    }
    let files: Vec<EslintFileReport> =
        serde_json::from_slice(bytes).map_err(|_| "eslint_report_invalid")?;
    let mut observed = BTreeSet::new();
    let mut findings = Vec::new();
    let (mut errors, mut warnings, mut suppressed_count, mut message_count) =
        (0_u64, 0_u64, 0_u64, 0_usize);
    let mut investigation = None;
    for file in files {
        if !valid_path(&file.file_path)
            || !expected.contains(file.file_path.as_str())
            || !observed.insert(file.file_path.clone())
        {
            return Err("eslint_report_scope_mismatch");
        }
        message_count = message_count
            .checked_add(file.messages.len())
            .and_then(|n| n.checked_add(file.suppressed_messages.len()))
            .ok_or("eslint_report_size_invalid")?;
        if message_count > 100_000 {
            return Err("eslint_report_size_invalid");
        }
        let (mut file_errors, mut file_warnings, mut file_fatal) = (0_u64, 0_u64, 0_u64);
        for message in file.messages {
            validate_message(&message)?;
            if message.severity == 2 {
                file_errors += 1;
            } else {
                file_warnings += 1;
            }
            if message.fatal {
                if message.severity != 2 {
                    return Err("eslint_report_counts_invalid");
                }
                file_fatal += 1;
                investigation = Some("eslint_parser_or_configuration_diagnostic");
            } else if let Some(rule) = message.rule_id.filter(|s| !s.is_empty()) {
                let (Some(line), Some(column)) = (message.line, message.column) else {
                    return Err("eslint_rule_location_invalid");
                };
                if line == 0 || column == 0 {
                    return Err("eslint_rule_location_invalid");
                }
                findings.push(EslintDiagnostic {
                    path: file.file_path.clone(),
                    rule_id: rule,
                    severity: message.severity,
                    line,
                    column,
                    message: message.message,
                });
            } else if investigation.is_none() {
                investigation = Some("eslint_unattributed_diagnostic");
            }
        }
        if (file_errors, file_warnings, file_fatal)
            != (file.error_count, file.warning_count, file.fatal_error_count)
            || file.fixable_error_count > file_errors
            || file.fixable_warning_count > file_warnings
        {
            return Err("eslint_report_counts_invalid");
        }
        errors += file_errors;
        warnings += file_warnings;
        for suppressed in &file.suppressed_messages {
            validate_message(suppressed)?;
        }
        suppressed_count += file.suppressed_messages.len() as u64;
    }
    if observed.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err("eslint_report_scope_mismatch");
    }
    let expected_exit = i32::from(errors > 0 || max_warnings.is_some_and(|max| warnings > max));
    if exit_code != Some(expected_exit) {
        return Err("eslint_exit_report_conflict");
    }
    let reason =
        investigation.or((suppressed_count > 0).then_some("eslint_suppression_requires_review"));
    Ok(EslintParsed {
        local_coherent: reason.is_none(),
        findings,
        suppressed_count,
        reason,
    })
}

/// 核对具体稳定 ESLint 10 版本字符串；相同声明不证明工具身份或原生兼容。
#[must_use]
pub fn eslint_report_version_matches(expected: &str, observed: &str) -> bool {
    expected == observed
        && semver::Version::parse(expected).is_ok_and(|version| {
            version.major == 10 && version.pre.is_empty() && version.build.is_empty()
        })
}

fn valid_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.chars().any(char::is_control)
        && !value
            .split(std::path::MAIN_SEPARATOR)
            .any(|part| matches!(part, "." | ".."))
        && path.is_absolute()
        && path.components().all(|c| {
            matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )
        })
}
fn validate_message(message: &EslintMessageReport) -> Result<(), &'static str> {
    if !matches!(message.severity, 1 | 2)
        || message.message.is_empty()
        || message.message.len() > 4096
        || message
            .rule_id
            .as_ref()
            .is_some_and(|r| r.len() > 512 || r.chars().any(char::is_control))
    {
        Err("eslint_message_invalid")
    } else {
        Ok(())
    }
}
