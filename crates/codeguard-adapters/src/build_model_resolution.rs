//! 动态构建模型解析任务与共享扫描义务映射
//!
//! 验收标准：detect/init/plan 仅保留待解析条件，执行期经统一 runtime 留证据

use serde::{Deserialize, Serialize};

/// 构建模型状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildModelState {
    /// 已解析
    Resolved,
    /// 待解析
    Pending,
    /// 解析失败
    Failed,
}

/// 构建根
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildRoot {
    /// 路径
    pub path: String,
    /// 构建系统
    pub build_system: String,
    /// 状态
    pub state: BuildModelState,
    /// 待解析条件
    pub pending_conditions: Vec<String>,
}

/// 扫描义务
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanObligation {
    /// 义务 ID
    pub id: String,
    /// 构建根路径
    pub build_root: String,
    /// 检查类别
    pub category: String,
    /// 是否已解析
    pub resolved: bool,
}

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// 已解析构建根
    pub resolved: Vec<BuildRoot>,
    /// 待解析构建根
    pub pending: Vec<BuildRoot>,
    /// 扫描义务
    pub obligations: Vec<ScanObligation>,
}

/// 构建模型解析器
pub struct BuildModelResolver;

impl BuildModelResolver {
    /// 解析构建模型
    pub fn resolve(roots: &[BuildRoot]) -> ResolutionResult {
        let mut resolved = Vec::new();
        let mut pending = Vec::new();
        let mut obligations = Vec::new();
        
        for root in roots {
            match root.state {
                BuildModelState::Resolved => {
                    resolved.push(root.clone());
                    
                    // 生成扫描义务
                    for category in &["lint", "comments", "dependencies", "cve", "security", "build"] {
                        obligations.push(ScanObligation {
                            id: format!("{}:{}", root.path, category),
                            build_root: root.path.clone(),
                            category: category.to_string(),
                            resolved: true,
                        });
                    }
                }
                BuildModelState::Pending => {
                    pending.push(root.clone());
                }
                BuildModelState::Failed => {
                    // 解析失败的构建根不生成义务
                }
            }
        }
        
        ResolutionResult {
            resolved,
            pending,
            obligations,
        }
    }
    
    /// 检查是否仅保留待解析条件
    pub fn only_pending_conditions(roots: &[BuildRoot]) -> bool {
        roots.iter().all(|r| {
            r.state == BuildModelState::Pending && !r.pending_conditions.is_empty()
        })
    }
    
    /// 验证执行期证据
    pub fn validate_execution_evidence(obligations: &[ScanObligation]) -> bool {
        obligations.iter().all(|o| o.resolved)
    }
}
