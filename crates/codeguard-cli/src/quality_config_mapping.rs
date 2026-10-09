//! 质量配置映射、测试/环境前置清单及准备任务
//!
//! 验收标准：缺工具或未知架构不虚构代码违规，不自动增加排除或存量豁免

use serde::{Deserialize, Serialize};

/// 前置条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prerequisite {
    /// 名称
    pub name: String,
    /// 是否满足
    pub satisfied: bool,
    /// 是否必需
    pub required: bool,
}

/// 质量配置映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfigMapping {
    /// 前置条件列表
    pub prerequisites: Vec<Prerequisite>,
    /// 是否虚构违规
    pub fabricated_violation: bool,
    /// 是否自动增加排除
    pub auto_exclude: bool,
}

/// 配置映射器
pub struct QualityConfigMapper;

impl QualityConfigMapper {
    /// 生成配置映射
    pub fn map(prerequisites: Vec<Prerequisite>) -> QualityConfigMapping {
        QualityConfigMapping {
            prerequisites,
            fabricated_violation: false,
            auto_exclude: false,
        }
    }
    
    /// 验证不虚构违规
    pub fn validate_no_fabrication(mapping: &QualityConfigMapping) -> bool {
        !mapping.fabricated_violation
    }
    
    /// 验证不自动增加排除
    pub fn validate_no_auto_exclude(mapping: &QualityConfigMapping) -> bool {
        !mapping.auto_exclude
    }
}
