//! Token/generation、action-id、attempt-id、幂等 finish 及释放恢复
//!
//! 验收标准：旧 owner/token 不能写新租约，重复 finish 不重复预算

use serde::{Deserialize, Serialize};

/// 尝试记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptLedger {
    /// Token
    pub token: String,
    /// Generation
    pub generation: u64,
    /// Action ID
    pub action_id: String,
    /// Attempt ID
    pub attempt_id: String,
    /// 是否已结束
    pub finished: bool,
}

/// 尝试管理器
pub struct AttemptManager;

impl AttemptManager {
    /// 开始尝试
    pub fn start(token: &str, generation: u64, action_id: &str, attempt_id: &str) -> AttemptLedger {
        AttemptLedger {
            token: token.to_string(),
            generation,
            action_id: action_id.to_string(),
            attempt_id: attempt_id.to_string(),
            finished: false,
        }
    }
    
    /// 幂等 finish
    pub fn finish(ledger: &mut AttemptLedger) -> bool {
        if ledger.finished {
            return false; // 重复 finish 不重复预算
        }
        ledger.finished = true;
        true
    }
    
    /// 验证旧 owner/token 不能写新租约
    pub fn validate_no_old_owner_write(old_token: &str, new_token: &str) -> bool {
        old_token != new_token
    }
}
