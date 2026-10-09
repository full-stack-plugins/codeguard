//! 执行前后内容身份复核和源码副作用检测
//!
//! 验收标准：并发编辑或检查器改源码导致 incomplete



/// 身份复核结果
#[derive(Debug, Clone)]
pub struct IdentityCheckResult {
    /// 是否一致
    pub consistent: bool,
    /// 是否有副作用
    pub has_side_effect: bool,
    /// 原因
    pub reason: Option<String>,
}

/// 内容身份检查器
pub struct ContentIdentityChecker;

impl ContentIdentityChecker {
    /// 复核内容身份
    pub fn check(before_hash: &str, after_hash: &str) -> IdentityCheckResult {
        let consistent = before_hash == after_hash;
        IdentityCheckResult {
            consistent,
            has_side_effect: !consistent,
            reason: if consistent { None } else { Some("content_changed".into()) },
        }
    }
    
    /// 验证并发编辑导致 incomplete
    pub fn validate_concurrent_edit_incomplete(result: &IdentityCheckResult) -> bool {
        result.has_side_effect && !result.consistent
    }
}
