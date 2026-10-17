//! Whitelist list/explain/propose 命令
//!
//! 验收标准：propose 只从本轮完整原生 finding 生成候选，缺任何身份报告未完成

use serde::{Deserialize, Serialize};

/// 候选
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistCandidate {
    /// 规则 ID
    pub rule_id: String,
    /// 目标
    pub target: String,
    /// 是否完整身份
    pub has_complete_identity: bool,
}

/// Propose 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeResult {
    /// 候选列表
    pub candidates: Vec<WhitelistCandidate>,
    /// 是否完成
    pub complete: bool,
    /// 缺失身份
    pub missing_identity: Option<String>,
}

/// Whitelist 命令处理器
pub struct WhitelistCommands;

impl WhitelistCommands {
    /// list
    pub fn list(candidates: &[WhitelistCandidate]) -> Vec<String> {
        candidates.iter().map(|c| c.rule_id.clone()).collect()
    }
    
    /// explain
    pub fn explain(candidate: &WhitelistCandidate) -> String {
        format!("Rule {}: target={}, identity={}", candidate.rule_id, candidate.target, candidate.has_complete_identity)
    }
    
    /// propose
    pub fn propose(findings: &[(String, String, bool)]) -> ProposeResult {
        let candidates: Vec<WhitelistCandidate> = findings.iter()
            .map(|(rule, target, has_identity)| WhitelistCandidate {
                rule_id: rule.clone(),
                target: target.clone(),
                has_complete_identity: *has_identity,
            })
            .collect();
        
        let missing = findings.iter()
            .find(|(_, _, has_identity)| !*has_identity)
            .map(|(rule, _, _)| rule.clone());
        
        ProposeResult {
            candidates,
            complete: missing.is_none(),
            missing_identity: missing,
        }
    }
}
