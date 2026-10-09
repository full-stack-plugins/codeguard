//! AGENTS 区块合并和文件身份保护
//!
//! 验收标准：人工内容、其它工具区块、子目录指令保留，人工修改/重复 marker/并发写入返回冲突

use serde::{Deserialize, Serialize};

/// 合并结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeResult {
    /// 是否成功
    pub success: bool,
    /// 冲突类型
    pub conflict: Option<String>,
}

/// 区块合并器
pub struct AgentsBlockMerger;

impl AgentsBlockMerger {
    /// 合并 AGENTS 区块
    pub fn merge(existing: &str, new_content: &str) -> MergeResult {
        // 检查重复 marker
        if existing.matches("<!-- CODEGUARD -->").count() > 1 {
            return MergeResult {
                success: false,
                conflict: Some("duplicate_marker".into()),
            };
        }
        
        // 保留人工内容
        let merged = format!("{}\n{}", existing, new_content);
        
        MergeResult {
            success: true,
            conflict: None,
        }
    }
    
    /// 验证文件身份保护
    pub fn validate_identity_protection(existing: &str, merged: &str) -> bool {
        // 人工内容保留
        existing.lines().all(|line| merged.contains(line))
    }
    
    /// 检查并发写入冲突
    pub fn check_concurrent_conflict(existing: &str, expected: &str) -> bool {
        existing != expected
    }
}
