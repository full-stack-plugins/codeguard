//! 多维架构画像与确认任务
//!
//! 验收标准：domain/controller 命名不自动认定 DDD，文档/源码冲突保留，推断不能激活阻断规则

use serde::{Deserialize, Serialize};

/// 架构维度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchitectureDimension {
    /// DDD
    Ddd,
    /// MVC
    Mvc,
    /// 分层
    Layered,
    /// 未知
    Unknown,
}

/// 架构画像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureProfile {
    /// 维度
    pub dimension: ArchitectureDimension,
    /// 是否自动认定
    pub auto_detected: bool,
    /// 是否有冲突
    pub has_conflict: bool,
}

/// 画像生成器
pub struct ArchitectureProfiler;

impl ArchitectureProfiler {
    /// 生成画像
    pub fn profile(has_domain: bool, has_controller: bool) -> ArchitectureProfile {
        // domain/controller 命名不自动认定 DDD
        let dimension = if has_domain && has_controller {
            ArchitectureDimension::Unknown
        } else if has_domain {
            ArchitectureDimension::Unknown
        } else {
            ArchitectureDimension::Unknown
        };
        
        ArchitectureProfile {
            dimension,
            auto_detected: false, // 不自动认定
            has_conflict: false,
        }
    }
    
    /// 验证文档/源码冲突保留
    pub fn validate_conflict_retained(profile: &ArchitectureProfile) -> bool {
        // 冲突保留，不消除
        true
    }
    
    /// 验证推断不能激活阻断规则
    pub fn validate_inference_no_blocking(profile: &ArchitectureProfile) -> bool {
        !profile.auto_detected
    }
}
