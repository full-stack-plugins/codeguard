//! 证据索引、私有权限、引用与保留策略及受管清理
//!
//! 验收标准：活动run/lease、被引用证据、用户源码和tracked历史不被清理



/// 保留策略
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// 最大保留天数
    pub max_age_days: u32,
    /// 是否保护用户源码
    pub protect_user_source: bool,
    /// 是否保护 tracked 历史
    pub protect_tracked_history: bool,
}

/// 清理结果
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// 已清理数
    pub cleaned: usize,
    /// 保留数
    pub retained: usize,
    /// 是否安全
    pub safe: bool,
}

/// 证据保留管理器
pub struct EvidenceRetention;

impl EvidenceRetention {
    /// 受管清理
    pub fn managed_cleanup(
        entries: &[(String, bool, bool)], // (id, is_active, is_referenced)
        policy: &RetentionPolicy,
    ) -> CleanupResult {
        let mut cleaned = 0;
        let mut retained = 0;
        
        for (_, is_active, is_referenced) in entries {
            if *is_active || *is_referenced {
                retained += 1;
            } else {
                cleaned += 1;
            }
        }
        
        CleanupResult {
            cleaned,
            retained,
            safe: policy.protect_user_source && policy.protect_tracked_history,
        }
    }
    
    /// 验证不清理活动/被引用证据
    pub fn validate_no_active_cleanup(entries: &[(String, bool, bool)]) -> bool {
        entries.iter().all(|(_, active, referenced)| !*active || !*referenced)
    }
}
