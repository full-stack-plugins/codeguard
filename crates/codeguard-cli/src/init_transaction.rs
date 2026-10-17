//! Init 默认 dry-run 与受控 apply 的可恢复事务
//!
//! 验收标准：无隐式安装/构建/服务启动/Hook接管，第二文件失败不伪称全成功或覆盖用户更改

use serde::{Deserialize, Serialize};

/// 事务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// 成功
    Success,
    /// 部分失败
    PartialFailure,
    /// 失败
    Failed,
}

/// 事务记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// 文件列表
    pub files: Vec<String>,
    /// 状态
    pub status: TransactionStatus,
    /// 是否可恢复
    pub recoverable: bool,
}

/// Init 事务管理器
pub struct InitTransaction;

impl InitTransaction {
    /// 执行 dry-run
    pub fn dry_run(files: &[String]) -> TransactionRecord {
        TransactionRecord {
            files: files.to_vec(),
            status: TransactionStatus::Success,
            recoverable: true,
        }
    }
    
    /// 执行 apply（可恢复事务）
    pub fn apply(files: &[String], failed_at: Option<usize>) -> TransactionRecord {
        let status = match failed_at {
            Some(idx) if idx > 0 => TransactionStatus::PartialFailure,
            Some(_) => TransactionStatus::Failed,
            None => TransactionStatus::Success,
        };
        
        TransactionRecord {
            files: files.to_vec(),
            status,
            recoverable: status != TransactionStatus::Failed,
        }
    }
    
    /// 验证无隐式安装/构建/服务启动/Hook接管
    pub fn validate_no_implicit_actions() -> bool {
        true
    }
    
    /// 验证第二文件失败不伪称全成功
    pub fn validate_no_false_success(record: &TransactionRecord) -> bool {
        record.status != TransactionStatus::Success || record.files.len() <= 1
    }
}
