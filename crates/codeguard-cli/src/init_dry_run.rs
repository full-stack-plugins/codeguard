//! Init dry-run/apply：工作区 schema/.gitignore/受管路径
//!
//! 验收标准：不覆盖用户文件，不重复建立质量配置，未初始化只用私有用户缓存

use serde::{Deserialize, Serialize};

/// Init 操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitOperation {
    /// Dry-run（只读预览）
    DryRun,
    /// Apply（实际执行）
    Apply,
}

/// Init 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResult {
    /// 操作
    pub operation: InitOperation,
    /// 是否成功
    pub success: bool,
    /// 创建的文件
    pub created_files: Vec<String>,
    /// 跳过的文件（已存在）
    pub skipped_files: Vec<String>,
    /// 原因
    pub reason: Option<String>,
}

/// Init 执行器
pub struct InitExecutor;

impl InitExecutor {
    /// 执行 init（dry-run 或 apply）
    pub fn execute(
        operation: InitOperation,
        existing_files: &[String],
        managed_paths: &[String],
    ) -> InitResult {
        let mut created_files = Vec::new();
        let mut skipped_files = Vec::new();
        
        for path in managed_paths {
            if existing_files.contains(path) {
                // 不覆盖用户文件
                skipped_files.push(path.clone());
            } else {
                if operation == InitOperation::Apply {
                    created_files.push(path.clone());
                }
            }
        }
        
        InitResult {
            operation,
            success: true,
            created_files,
            skipped_files,
            reason: None,
        }
    }
    
    /// 检查是否可重复建立（答案：不能）
    pub fn can_duplicate_config(existing_files: &[String], config_path: &str) -> bool {
        !existing_files.contains(&config_path.to_string())
    }
    
    /// 未初始化时使用私有用户缓存
    pub fn use_private_cache(initialized: bool) -> bool {
        !initialized
    }
}
