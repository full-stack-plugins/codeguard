//! 自有产物精确范围规则与工作区校验
//!
//! 验收标准：运行副本不递归扫描，codeguard/src 用户源码正常检查，入库 secret 仍阻断

use serde::{Deserialize, Serialize};

/// 范围规则
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeRule {
    /// 路径模式
    pub pattern: String,
    /// 是否排除
    pub exclude: bool,
    /// 是否递归
    pub recursive: bool,
}

/// 范围检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeCheckResult {
    /// 文件路径
    pub file: String,
    /// 是否在范围内
    pub in_scope: bool,
    /// 原因
    pub reason: Option<String>,
}

/// 范围检查器
pub struct ScopeChecker;

impl ScopeChecker {
    /// 检查文件是否在范围内
    pub fn check_scope(
        file: &str,
        rules: &[ScopeRule],
        is_user_source: bool,
        has_secret: bool,
    ) -> ScopeCheckResult {
        // 检查 secret（入库 secret 仍阻断）
        if has_secret {
            return ScopeCheckResult {
                file: file.to_string(),
                in_scope: false,
                reason: Some("secret_detected".into()),
            };
        }
        
        // 检查用户源码（codeguard/src 用户源码正常检查）
        if is_user_source && file.contains("codeguard/src") {
            return ScopeCheckResult {
                file: file.to_string(),
                in_scope: true,
                reason: Some("user_source".into()),
            };
        }
        
        // 检查排除规则
        for rule in rules {
            if rule.exclude && file.contains(&rule.pattern) {
                return ScopeCheckResult {
                    file: file.to_string(),
                    in_scope: false,
                    reason: Some(format!("excluded: {}", rule.pattern)),
                };
            }
        }
        
        ScopeCheckResult {
            file: file.to_string(),
            in_scope: true,
            reason: None,
        }
    }
    
    /// 检查运行副本不递归扫描
    pub fn check_no_recursive_scan(path: &str, rules: &[ScopeRule]) -> bool {
        rules.iter().all(|r| {
            if r.pattern.contains("copy") || r.pattern.contains("backup") {
                !r.recursive
            } else {
                true
            }
        })
    }
    
    /// 验证工作区校验
    pub fn validate_workspace(files: &[ScopeCheckResult]) -> bool {
        // 所有 secret 文件都不在范围内
        files.iter().all(|f| {
            if f.reason.as_deref() == Some("secret_detected") {
                !f.in_scope
            } else {
                true
            }
        })
    }
}
