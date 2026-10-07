//! 基线分类：基线仅分类 new/existing
//!
//! 验收标准：未修改文件中的存量违规仍阻断，基线失败不消除 finding

use serde::{Deserialize, Serialize};

/// 发现分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingClass {
    /// 新发现
    New,
    /// 已存在（基线中）
    Existing,
}

/// 基线条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineEntry {
    /// 文件路径
    pub file: String,
    /// 规则 ID
    pub rule: String,
    /// 行号
    pub line: u32,
    /// 指纹（用于匹配）
    pub fingerprint: String,
}

/// 基线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// 基线条目
    pub entries: Vec<BaselineEntry>,
    /// 基线版本
    pub version: String,
}

/// 分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// 分类后的发现
    pub findings: Vec<ClassifiedFinding>,
    /// 新发现数
    pub new_count: usize,
    /// 已存在数
    pub existing_count: usize,
}

/// 已分类发现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedFinding {
    /// 文件路径
    pub file: String,
    /// 规则 ID
    pub rule: String,
    /// 行号
    pub line: u32,
    /// 分类
    pub class: FindingClass,
    /// 指纹
    pub fingerprint: String,
}

/// 基线分类器
pub struct BaselineClassifier;

impl BaselineClassifier {
    /// 分类发现
    /// 
    /// 验收标准：基线仅分类 new/existing，未修改文件中的存量违规仍阻断
    pub fn classify(
        findings: &[(String, String, u32)],
        baseline: &Baseline,
    ) -> ClassificationResult {
        let mut classified = Vec::new();
        let mut new_count = 0;
        let mut existing_count = 0;
        
        for (file, rule, line) in findings {
            let fingerprint = Self::compute_fingerprint(file, rule, *line);
            
            // 检查是否在基线中
            let is_existing = baseline.entries.iter().any(|entry| {
                entry.fingerprint == fingerprint
                    || (entry.file == *file && entry.rule == *rule && entry.line == *line)
            });
            
            let class = if is_existing {
                existing_count += 1;
                FindingClass::Existing
            } else {
                new_count += 1;
                FindingClass::New
            };
            
            classified.push(ClassifiedFinding {
                file: file.clone(),
                rule: rule.clone(),
                line: *line,
                class,
                fingerprint,
            });
        }
        
        ClassificationResult {
            findings: classified,
            new_count,
            existing_count,
        }
    }
    
    /// 计算指纹
    fn compute_fingerprint(file: &str, rule: &str, line: u32) -> String {
        format!("{}:{}:{}", file, rule, line)
    }
    
    /// 验证基线（基线失败不消除 finding）
    pub fn validate_baseline(baseline: &Baseline) -> Result<(), String> {
        if baseline.entries.is_empty() {
            return Err("baseline_empty".into());
        }
        
        // 检查重复条目
        let mut seen = std::collections::HashSet::new();
        for entry in &baseline.entries {
            if !seen.insert(entry.fingerprint.clone()) {
                return Err(format!("duplicate_entry: {}", entry.fingerprint));
            }
        }
        
        Ok(())
    }
    
    /// 创建基线
    pub fn create_baseline(findings: &[(String, String, u32)]) -> Baseline {
        let entries = findings.iter()
            .map(|(file, rule, line)| BaselineEntry {
                file: file.clone(),
                rule: rule.clone(),
                line: *line,
                fingerprint: Self::compute_fingerprint(file, rule, *line),
            })
            .collect();
        
        Baseline {
            entries,
            version: "1.0.0".into(),
        }
    }
}
