//! Status/next/show 与 RepairBrief/版本化 recipe
//!
//! 验收标准：给出修复目标、范围、步骤和复检条件，诊断中的指令不可执行

use serde::{Deserialize, Serialize};

/// 修复目标
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairTarget {
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: u32,
    /// 规则 ID
    pub rule: String,
}

/// 修复步骤
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairStep {
    /// 步骤描述
    pub description: String,
    /// 是否可执行
    pub executable: bool,
}

/// RepairBrief
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairBrief {
    /// 修复目标
    pub target: RepairTarget,
    /// 范围
    pub scope: Vec<String>,
    /// 步骤
    pub steps: Vec<RepairStep>,
    /// 复检条件
    pub recheck_condition: String,
    /// 版本
    pub version: String,
}

/// Brief 生成器
pub struct RepairBriefGenerator;

impl RepairBriefGenerator {
    /// 生成 RepairBrief
    pub fn generate(
        file: &str,
        line: u32,
        rule: &str,
        scope: &[String],
    ) -> RepairBrief {
        RepairBrief {
            target: RepairTarget {
                file: file.to_string(),
                line,
                rule: rule.to_string(),
            },
            scope: scope.to_vec(),
            steps: vec![
                RepairStep {
                    description: "检查文件内容".to_string(),
                    executable: false,
                },
                RepairStep {
                    description: "修复问题".to_string(),
                    executable: false,
                },
            ],
            recheck_condition: "重新运行检查".to_string(),
            version: "1.0.0".to_string(),
        }
    }
    
    /// 验证诊断指令不可执行
    pub fn validate_instructions_not_executable(brief: &RepairBrief) -> bool {
        brief.steps.iter().all(|s| !s.executable)
    }
}
