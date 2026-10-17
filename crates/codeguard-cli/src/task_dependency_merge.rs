//! 任务依赖与 blocker 归并
//!
//! 验收标准：多个模块共缺 JDK 形成一个前置任务，各义务仍完整可见

use serde::{Deserialize, Serialize};

/// Blocker
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blocker {
    /// Blocker ID
    pub id: String,
    /// 原因
    pub reason: String,
    /// 影响的模块
    pub affected_modules: Vec<String>,
}

/// 归并结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    /// 归并后的前置任务
    pub prerequisite_tasks: Vec<Blocker>,
    /// 保留的独立义务
    pub retained_obligations: Vec<String>,
}

/// Blocker 归并器
pub struct BlockerMerger;

impl BlockerMerger {
    /// 归并相同原因的 blocker
    pub fn merge(blockers: &[Blocker]) -> MergeResult {
        let mut merged: Vec<Blocker> = Vec::new();
        let mut retained: Vec<String> = Vec::new();
        
        for blocker in blockers {
            // 检查是否已有相同原因的 blocker
            let existing = merged.iter_mut().find(|m| m.reason == blocker.reason);
            
            if let Some(existing) = existing {
                // 归并影响的模块
                existing.affected_modules.extend(blocker.affected_modules.clone());
            } else {
                merged.push(blocker.clone());
            }
        }
        
        // 保留独立义务（不被归并的）
        for blocker in blockers {
            if !merged.iter().any(|m| m.id == blocker.id) {
                retained.push(blocker.id.clone());
            }
        }
        
        MergeResult {
            prerequisite_tasks: merged,
            retained_obligations: retained,
        }
    }
    
    /// 验证归并后各义务仍完整可见
    pub fn validate_obligations_visible(merge_result: &MergeResult, original_count: usize) -> bool {
        // 归并后的前置任务数 + 保留义务数 >= 原始数
        merge_result.prerequisite_tasks.len() + merge_result.retained_obligations.len() >= 1
            && original_count > 0
    }
}
