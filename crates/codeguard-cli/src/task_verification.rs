//! Task verify 证据关闭与重开
//!
//! 验收标准：工具超时、加 ignore、移动到排除目录不自动 resolved，真修复有完整身份绑定

use serde::{Deserialize, Serialize};

/// 验证结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// 已修复
    Resolved,
    /// 仍存在
    StillPresent,
    /// 未完成
    Incomplete,
    /// 复发
    Recurrence,
}

/// 验证记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    /// 任务 ID
    pub task_id: String,
    /// 结果
    pub result: VerificationResult,
    /// 身份绑定
    pub identity_binding: String,
    /// 是否自动关闭
    pub auto_closed: bool,
}

/// Task verifier
pub struct TaskVerification;

impl TaskVerification {
    /// 验证任务
    pub fn verify(task_id: &str, result: VerificationResult, identity_binding: &str) -> VerificationRecord {
        let auto_closed = result == VerificationResult::Resolved && !identity_binding.is_empty();
        
        VerificationRecord {
            task_id: task_id.to_string(),
            result,
            identity_binding: identity_binding.to_string(),
            auto_closed,
        }
    }
    
    /// 检查是否可关闭
    pub fn can_close(record: &VerificationRecord) -> bool {
        record.result == VerificationResult::Resolved && !record.identity_binding.is_empty()
    }
    
    /// 检查是否可重开
    pub fn can_reopen(record: &VerificationRecord) -> bool {
        record.result == VerificationResult::Recurrence || record.result == VerificationResult::StillPresent
    }
}
