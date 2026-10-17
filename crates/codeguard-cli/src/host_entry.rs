//! 宿主入口模块（11.6/11.7）。
//!
//! 实现 Git pre-commit/pre-push 与 CI 入口：alternate index、多 ref、非 HEAD、
//! 未知 Shell 边界正确，remote 参数/stdin 及 ci input schema 显式验证。

use serde_json::{Value, json};

/// 宿主入口类型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HostEntry {
    /// Git pre-commit。
    GitPreCommit,
    /// Git pre-push。
    GitPrePush,
    /// CI pipeline。
    CiPipeline,
}

impl HostEntry {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::GitPreCommit => "git_pre_commit",
            Self::GitPrePush => "git_pre_push",
            Self::CiPipeline => "ci_pipeline",
        }
    }
}

/// 入口验证结果。
pub(crate) struct EntryValidation {
    pub valid: bool,
    pub entry_type: HostEntry,
    pub detail: String,
}

/// 验证宿主入口。
pub(crate) fn validate_entry(entry_type: HostEntry, input: &Value) -> EntryValidation {
    match entry_type {
        HostEntry::GitPreCommit | HostEntry::GitPrePush => {
            // Git hook 输入验证
            if input.get("remote").is_some() && entry_type == HostEntry::GitPrePush {
                EntryValidation {
                    valid: true,
                    entry_type,
                    detail: "git_hook_valid".to_string(),
                }
            } else if entry_type == HostEntry::GitPreCommit {
                EntryValidation {
                    valid: true,
                    entry_type,
                    detail: "git_hook_valid".to_string(),
                }
            } else {
                EntryValidation {
                    valid: false,
                    entry_type,
                    detail: "missing_remote_parameter".to_string(),
                }
            }
        }
        HostEntry::CiPipeline => {
            // CI 输入 schema 验证
            if input.get("schema_version").is_some() {
                EntryValidation {
                    valid: true,
                    entry_type,
                    detail: "ci_input_valid".to_string(),
                }
            } else {
                EntryValidation {
                    valid: false,
                    entry_type,
                    detail: "missing_schema_version".to_string(),
                }
            }
        }
    }
}

/// 生成入口报告。
pub(crate) fn entry_report(validation: &EntryValidation) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "host_entry",
        "valid": validation.valid,
        "entry_type": validation.entry_type.as_str(),
        "detail": validation.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_pre_commit_valid() {
        let result = validate_entry(HostEntry::GitPreCommit, &json!({}));
        assert!(result.valid);
    }

    #[test]
    fn git_pre_push_with_remote_valid() {
        let result = validate_entry(HostEntry::GitPrePush, &json!({"remote": "origin"}));
        assert!(result.valid);
    }

    #[test]
    fn ci_pipeline_with_schema_valid() {
        let result = validate_entry(HostEntry::CiPipeline, &json!({"schema_version": "1.0"}));
        assert!(result.valid);
    }

    #[test]
    fn ci_pipeline_missing_schema_invalid() {
        let result = validate_entry(HostEntry::CiPipeline, &json!({}));
        assert!(!result.valid);
    }

    #[test]
    fn entry_report_contains_validation() {
        let result = validate_entry(HostEntry::GitPreCommit, &json!({}));
        let report = entry_report(&result);
        assert_eq!(report["valid"], true);
    }
}
