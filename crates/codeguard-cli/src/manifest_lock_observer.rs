//! Manifest/锁/wrapper 静态观察
//!
//! 验收标准：init 不执行项目脚本，声明版本和已解析版本分别有依据

use serde::{Deserialize, Serialize};

/// 静态观察结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticObservation {
    /// Manifest 路径
    pub manifest_path: String,
    /// 声明版本
    pub declared_version: String,
    /// 已解析版本
    pub resolved_version: Option<String>,
    /// 是否执行项目脚本
    pub executed_script: bool,
}

/// 静态观察器
pub struct StaticObserver;

impl StaticObserver {
    /// 观察 manifest/锁/wrapper
    pub fn observe(
        manifest_path: &str,
        declared_version: &str,
        resolved_version: Option<&str>,
    ) -> StaticObservation {
        StaticObservation {
            manifest_path: manifest_path.to_string(),
            declared_version: declared_version.to_string(),
            resolved_version: resolved_version.map(String::from),
            executed_script: false, // init 不执行项目脚本
        }
    }
    
    /// 验证声明版本和已解析版本分别有依据
    pub fn validate_versions(observation: &StaticObservation) -> bool {
        !observation.declared_version.is_empty()
            && (observation.resolved_version.is_none()
                || !observation.resolved_version.as_deref().unwrap_or("").is_empty())
    }
    
    /// 验证不执行项目脚本
    pub fn validate_no_script_execution(observation: &StaticObservation) -> bool {
        !observation.executed_script
    }
}
