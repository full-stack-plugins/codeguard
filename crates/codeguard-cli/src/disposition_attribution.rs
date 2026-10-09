//! 处置归因与例外标签
//!
//! 验收标准：policy_resolved 不计代码修复，例外保留未解决事实和期限

use serde::{Deserialize, Serialize};

/// 处置类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    /// 代码修复
    Code,
    /// 依赖修复
    Dependency,
    /// 环境修复
    Environment,
    /// 目标修复
    Target,
    /// 策略解决
    Policy,
}

/// 例外标签
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionLabel {
    /// 例外 ID
    pub id: String,
    /// 到期时间
    pub expires_at: u64,
    /// 是否保留未解决事实
    pub retains_unresolved: bool,
}

/// 归因结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionResult {
    /// 处置类型
    pub disposition: Disposition,
    /// 是否计为代码修复
    pub counts_as_code_fix: bool,
    /// 例外标签
    pub exception: Option<ExceptionLabel>,
}

/// 归因器
pub struct DispositionAttributor;

impl DispositionAttributor {
    /// 归因处置
    pub fn attribute(disposition: Disposition, exception: Option<ExceptionLabel>) -> AttributionResult {
        // policy_resolved 不计代码修复
        let counts_as_code_fix = disposition != Disposition::Policy;
        
        AttributionResult {
            disposition,
            counts_as_code_fix,
            exception,
        }
    }
    
    /// 验证例外保留未解决事实和期限
    pub fn validate_exception(exception: &ExceptionLabel) -> bool {
        exception.retains_unresolved && exception.expires_at > 0
    }
}
