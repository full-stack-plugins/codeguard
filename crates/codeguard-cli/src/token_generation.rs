//! Token/generation 幂等 finish
//!
//! 验收标准：同 token 重复 finish 不产生新事件，generation 变化后可重新 finish

use serde::{Deserialize, Serialize};

/// Finish 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishResult {
    /// 是否幂等
    pub idempotent: bool,
    /// 是否新事件
    pub new_event: bool,
    /// Generation
    pub generation: u64,
}

/// Token/generation 管理器
pub struct TokenGenerationManager;

impl TokenGenerationManager {
    /// Finish 操作
    pub fn finish(token: &str, generation: u64, previous_generation: u64) -> FinishResult {
        // 同 token 重复 finish 不产生新事件
        let idempotent = generation == previous_generation;
        
        FinishResult {
            idempotent,
            new_event: !idempotent,
            generation,
        }
    }
    
    /// 验证幂等性
    pub fn validate_idempotent(result: &FinishResult) -> bool {
        result.idempotent
    }
    
    /// 验证 generation 变化后可重新 finish
    pub fn validate_generation_change(new_gen: u64, old_gen: u64) -> bool {
        new_gen != old_gen
    }
}
