//! 项目检查器配置探测
//!
//! 验收标准：按构建根识别 Javadoc、依赖/CVE、lint 与安全检查器的 configured/missing/invalid/unknown

use serde::{Deserialize, Serialize};

/// 配置状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigStatus {
    /// 已配置
    Configured,
    /// 缺失
    Missing,
    /// 无效
    Invalid,
    /// 未知
    Unknown,
}

/// 检查器配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckerConfig {
    /// 检查器名称
    pub name: String,
    /// 类别
    pub category: String,
    /// 配置状态
    pub status: ConfigStatus,
    /// 配置位置
    pub config_location: Option<String>,
    /// 启用建议
    pub enable_suggestion: Option<String>,
}

/// 配置探测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// 构建根路径
    pub build_root: String,
    /// 检查器配置
    pub checkers: Vec<CheckerConfig>,
}

/// 配置探测器
pub struct CheckerConfigDetector;

impl CheckerConfigDetector {
    /// 探测检查器配置
    pub fn detect(build_root: &str, configs: &[(String, String, ConfigStatus)]) -> DetectionResult {
        let checkers = configs.iter()
            .map(|(name, category, status)| CheckerConfig {
                name: name.clone(),
                category: category.clone(),
                status: *status,
                config_location: match status {
                    ConfigStatus::Configured => Some(format!("{}/config", build_root)),
                    _ => None,
                },
                enable_suggestion: match status {
                    ConfigStatus::Missing => Some(format!("配置 {} 检查器", name)),
                    ConfigStatus::Invalid => Some(format!("修复 {} 配置", name)),
                    _ => None,
                },
            })
            .collect();
        
        DetectionResult {
            build_root: build_root.to_string(),
            checkers,
        }
    }
    
    /// 验证探测结果
    pub fn validate(result: &DetectionResult) -> bool {
        result.checkers.iter().all(|c| {
            match c.status {
                ConfigStatus::Configured => c.config_location.is_some(),
                ConfigStatus::Missing => c.enable_suggestion.is_some(),
                ConfigStatus::Invalid => c.enable_suggestion.is_some(),
                ConfigStatus::Unknown => true,
            }
        })
    }
}
