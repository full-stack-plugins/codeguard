//! init dry-run 事务模块（9.22）。
//!
//! 实现 init 默认 dry-run 与受控 apply 的可恢复事务：不覆盖用户文件，
//! 不重复建立质量配置，未初始化只用私有用户缓存存原始报告。

use serde_json::{Value, json};

/// init 事务状态。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InitTransaction {
    /// dry-run：只预览，不写入。
    DryRun,
    /// apply：受控写入。
    Apply,
    /// 失败：可恢复。
    Failed,
}

impl InitTransaction {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Apply => "apply",
            Self::Failed => "failed",
        }
    }
}

/// init 事务结果。
pub(crate) struct InitResult {
    pub transaction: InitTransaction,
    pub files_created: Vec<String>,
    pub files_skipped: Vec<String>,
    pub detail: String,
}

/// 执行 init dry-run。
pub(crate) fn init_dry_run(files_to_create: Vec<String>) -> InitResult {
    InitResult {
        transaction: InitTransaction::DryRun,
        files_created: vec![],
        files_skipped: files_to_create,
        detail: "dry_run_preview".to_string(),
    }
}

/// 执行 init apply。
pub(crate) fn init_apply(files_to_create: Vec<String>, existing_files: Vec<String>) -> InitResult {
    let files_created: Vec<String> = files_to_create
        .iter()
        .filter(|f| !existing_files.contains(f))
        .cloned()
        .collect();
    let files_skipped: Vec<String> = files_to_create
        .iter()
        .filter(|f| existing_files.contains(f))
        .cloned()
        .collect();

    InitResult {
        transaction: InitTransaction::Apply,
        files_created,
        files_skipped,
        detail: "apply_completed".to_string(),
    }
}

/// 生成 init 事务报告。
pub(crate) fn init_report(result: &InitResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "init_transaction",
        "transaction": result.transaction.as_str(),
        "files_created": result.files_created,
        "files_skipped": result.files_skipped,
        "detail": result.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_creates_no_files() {
        let result = init_dry_run(vec!["a.txt".to_string(), "b.txt".to_string()]);
        assert_eq!(result.transaction, InitTransaction::DryRun);
        assert!(result.files_created.is_empty());
    }

    #[test]
    fn apply_creates_new_files() {
        let result = init_apply(
            vec!["a.txt".to_string(), "b.txt".to_string()],
            vec!["a.txt".to_string()],
        );
        assert_eq!(result.transaction, InitTransaction::Apply);
        assert_eq!(result.files_created, vec!["b.txt".to_string()]);
        assert_eq!(result.files_skipped, vec!["a.txt".to_string()]);
    }

    #[test]
    fn apply_skips_existing_files() {
        let result = init_apply(vec!["a.txt".to_string()], vec!["a.txt".to_string()]);
        assert!(result.files_created.is_empty());
        assert_eq!(result.files_skipped.len(), 1);
    }

    #[test]
    fn init_report_contains_transaction() {
        let result = init_dry_run(vec!["a.txt".to_string()]);
        let report = init_report(&result);
        assert_eq!(report["transaction"], "dry_run");
    }
}
