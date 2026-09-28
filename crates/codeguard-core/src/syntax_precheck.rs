use std::collections::BTreeSet;

use crate::{SyntaxFileObservation, SyntaxFileState, SyntaxPrecheckOutcome, SyntaxPrecheckStatus};

/// 按选定文件范围聚合语法初检；枚举中断、取消、未知版本和局部失败不能成为 clean。
/// 参数为每个已选文件的唯一观察、范围是否完成及取消状态；返回初检状态或输入契约错误。
pub fn assess_syntax_precheck(
    files: &[SyntaxFileObservation],
    scope_complete: bool,
    cancelled: bool,
) -> Result<SyntaxPrecheckOutcome, &'static str> {
    let mut paths = BTreeSet::new();
    let mut outcome = SyntaxPrecheckOutcome {
        status: SyntaxPrecheckStatus::NotRun,
        scope_complete,
        cancelled,
        selected_files: files.len(),
        checked_files: 0,
        incomplete_files: 0,
        unsupported_files: 0,
        unqualified_files: 0,
        truncated_files: 0,
        suspected_recoveries: 0,
    };
    for file in files {
        if !valid_relative_path(&file.path) || !paths.insert(file.path.as_str()) {
            return Err("syntax_scope_invalid_or_duplicate_path");
        }
        match &file.state {
            SyntaxFileState::Checked {
                recoveries,
                grammar_qualified,
                truncated,
            } => {
                outcome.checked_files += 1;
                outcome.suspected_recoveries = outcome
                    .suspected_recoveries
                    .checked_add(*recoveries)
                    .ok_or("syntax_recovery_count_overflow")?;
                if !grammar_qualified {
                    outcome.unqualified_files += 1;
                }
                if *truncated {
                    outcome.truncated_files += 1;
                }
            }
            SyntaxFileState::Incomplete { reason } => {
                if reason.is_empty() {
                    return Err("syntax_incomplete_reason_missing");
                }
                outcome.incomplete_files += 1;
            }
            SyntaxFileState::Unsupported { reason } => {
                if reason.is_empty() {
                    return Err("syntax_unsupported_reason_missing");
                }
                outcome.unsupported_files += 1;
            }
        }
    }
    outcome.status = if !scope_complete || cancelled {
        SyntaxPrecheckStatus::Incomplete
    } else if files.is_empty() {
        SyntaxPrecheckStatus::NotRun
    } else if outcome.unsupported_files == files.len() {
        SyntaxPrecheckStatus::Unsupported
    } else if outcome.incomplete_files > 0
        || outcome.unsupported_files > 0
        || outcome.unqualified_files > 0
        || outcome.truncated_files > 0
    {
        SyntaxPrecheckStatus::Incomplete
    } else if outcome.suspected_recoveries > 0 {
        SyntaxPrecheckStatus::SuspectedIssue
    } else {
        SyntaxPrecheckStatus::Clean
    };
    Ok(outcome)
}

fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
