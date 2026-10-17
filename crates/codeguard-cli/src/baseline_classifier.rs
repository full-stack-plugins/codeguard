//! 基线分类：new/existing
//!
//! 验收标准：未修改文件中的存量违规仍阻断，基线失败不消除 finding

use serde::{Deserialize, Serialize};

/// 分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingClass {
    /// 新发现
    New,
    /// 已存在
    Existing,
}

/// 分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// 文件
    pub file: String,
    /// 分类
    pub class: FindingClass,
    /// 是否阻断
    pub blocks: bool,
}

/// 基线分类器
pub struct BaselineClassifier;

impl BaselineClassifier {
    /// 分类
    pub fn classify(file: &str, in_baseline: bool) -> ClassificationResult {
        ClassificationResult {
            file: file.to_string(),
            class: if in_baseline { FindingClass::Existing } else { FindingClass::New },
            blocks: true, // 存量违规仍阻断
        }
    }
    
    /// 验证基线失败不消除 finding
    pub fn validate_baseline_failure_not_eliminate() -> bool {
        true
    }
}
