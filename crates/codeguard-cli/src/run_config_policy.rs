//! 运行配置与批准质量策略独立解析
//!
//! 验收标准：CLI/env 无法弱化 required/threshold/exclude

use serde::{Deserialize, Serialize};

/// 批准策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovedPolicy {
    /// 必需检查
    pub required: Vec<String>,
    /// 最低阈值
    pub min_threshold: f64,
    /// 排除路径
    pub exclude: Vec<String>,
}

/// 运行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// 超时
    pub timeout_ms: u64,
    /// 并发数
    pub jobs: usize,
}

/// 合并结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedConfig {
    /// 策略
    pub policy: ApprovedPolicy,
    /// 运行配置
    pub run: RunConfig,
    /// 是否有弱化尝试
    pub weakening_attempted: bool,
}

/// 配置解析器
pub struct RunConfigPolicyResolver;

impl RunConfigPolicyResolver {
    /// 解析配置
    pub fn resolve(policy: &ApprovedPolicy, cli: &RunConfig, env: &RunConfig) -> MergedConfig {
        let run = RunConfig {
            timeout_ms: if cli.timeout_ms > 0 { cli.timeout_ms } else { env.timeout_ms },
            jobs: if cli.jobs > 0 { cli.jobs } else { env.jobs },
        };
        
        MergedConfig {
            policy: policy.clone(),
            run,
            weakening_attempted: false,
        }
    }
    
    /// 验证无法弱化
    pub fn validate_no_weakening(merged: &MergedConfig, original: &ApprovedPolicy) -> bool {
        merged.policy == *original
    }
}
