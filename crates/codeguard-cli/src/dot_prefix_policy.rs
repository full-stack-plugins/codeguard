//! 点前缀默认策略及两项例外
//!
//! 验收标准：F18 全部成立，无未授权的新排除

use serde::{Deserialize, Serialize};

/// 策略条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyEntry {
    /// 路径模式
    pub pattern: String,
    /// 是否排除
    pub exclude: bool,
    /// 是否授权
    pub authorized: bool,
}

/// 策略结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// 是否有效
    pub valid: bool,
    /// 未授权排除数
    pub unauthorized_excludes: usize,
}

/// 点前缀策略
pub struct DotPrefixPolicy;

impl DotPrefixPolicy {
    /// 验证策略
    pub fn validate(entries: &[PolicyEntry]) -> PolicyResult {
        let unauthorized = entries.iter()
            .filter(|e| e.exclude && !e.authorized)
            .count();
        
        PolicyResult {
            valid: unauthorized == 0,
            unauthorized_excludes: unauthorized,
        }
    }
    
    /// 检查 F18 全部成立
    pub fn check_f18(entries: &[PolicyEntry]) -> bool {
        entries.iter().all(|e| e.authorized || !e.exclude)
    }
}
