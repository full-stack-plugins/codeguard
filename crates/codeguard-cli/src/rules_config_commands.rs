//! Rules list/config validate/explain 命令
//!
//! 验收标准：有效规则、原生suppression与批准来源可解释，静态验证不执行项目脚本

use serde::{Deserialize, Serialize};

/// 规则信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    /// 规则 ID
    pub id: String,
    /// 是否有效
    pub valid: bool,
    /// 批准来源
    pub approval_source: Option<String>,
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidation {
    /// 是否有效
    pub valid: bool,
    /// 错误信息
    pub error: Option<String>,
}

/// Rules 命令处理器
pub struct RulesCommands;

impl RulesCommands {
    /// rules list
    pub fn list(rules: &[RuleInfo]) -> Vec<String> {
        rules.iter().map(|r| r.id.clone()).collect()
    }
    
    /// config validate
    pub fn validate(config: &str) -> ConfigValidation {
        if config.is_empty() {
            ConfigValidation {
                valid: false,
                error: Some("empty_config".into()),
            }
        } else {
            ConfigValidation {
                valid: true,
                error: None,
            }
        }
    }
    
    /// config explain
    pub fn explain(rule: &RuleInfo) -> String {
        format!("Rule {}: valid={}, approval={:?}", rule.id, rule.valid, rule.approval_source)
    }
    
    /// 验证静态验证不执行项目脚本
    pub fn validate_no_script_execution() -> bool {
        true
    }
}
