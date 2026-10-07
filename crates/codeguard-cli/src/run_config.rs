//! 运行配置与批准质量策略独立解析
//!
//! 验收标准：CLI/env 无法弱化 required/threshold/exclude

use serde::{Deserialize, Serialize};

/// 批准质量策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovedPolicy {
    /// 必需检查类别
    pub required: Vec<String>,
    /// 最低阈值（Wilson 下界）
    pub min_threshold: f64,
    /// 排除路径
    pub exclude: Vec<String>,
    /// 策略版本
    pub version: String,
}

/// 运行配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunConfig {
    /// 运行配置版本
    pub version: String,
    /// 超时（毫秒）
    pub timeout_ms: u64,
    /// 并发数
    pub jobs: usize,
}

/// 配置合并结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergedConfig {
    /// 批准策略（不可弱化）
    pub policy: ApprovedPolicy,
    /// 运行配置（可覆盖）
    pub run: RunConfig,
    /// 是否有弱化尝试
    pub weakening_attempted: bool,
    /// 弱化详情
    pub weakening_details: Vec<String>,
}

/// 配置解析器
pub struct ConfigResolver;

impl ConfigResolver {
    /// 解析并合并配置
    /// 
    /// 验收标准：CLI/env 无法弱化 required/threshold/exclude
    pub fn resolve(
        approved_policy: &ApprovedPolicy,
        cli_overrides: &RunConfig,
        env_overrides: &RunConfig,
    ) -> MergedConfig {
        let mut weakening_details = Vec::new();
        
        // 运行配置合并（CLI 优先于 env）
        let run = RunConfig {
            version: cli_overrides.version.clone(),
            timeout_ms: if cli_overrides.timeout_ms > 0 {
                cli_overrides.timeout_ms
            } else {
                env_overrides.timeout_ms
            },
            jobs: if cli_overrides.jobs > 0 {
                cli_overrides.jobs
            } else {
                env_overrides.jobs
            },
        };
        
        // 检查是否有弱化尝试（CLI/env 不能修改 required/threshold/exclude）
        // 这里我们只接受 approved_policy，不接受来自 CLI/env 的弱化
        
        MergedConfig {
            policy: approved_policy.clone(),
            run,
            weakening_attempted: !weakening_details.is_empty(),
            weakening_details,
        }
    }
    
    /// 验证配置是否合法
    pub fn validate(config: &MergedConfig) -> Result<(), String> {
        // 验证必需检查类别不为空
        if config.policy.required.is_empty() {
            return Err("required_categories_empty".into());
        }
        
        // 验证阈值在合理范围
        if config.policy.min_threshold < 0.0 || config.policy.min_threshold > 1.0 {
            return Err("threshold_out_of_range".into());
        }
        
        // 验证超时合理
        if config.run.timeout_ms == 0 {
            return Err("timeout_must_be_positive".into());
        }
        
        Ok(())
    }
    
    /// 配置解释（config explain）
    pub fn explain(config: &MergedConfig) -> ConfigExplanation {
        ConfigExplanation {
            policy_version: config.policy.version.clone(),
            required_count: config.policy.required.len(),
            min_threshold: config.policy.min_threshold,
            exclude_count: config.policy.exclude.len(),
            timeout_ms: config.run.timeout_ms,
            jobs: config.run.jobs,
            weakening_attempted: config.weakening_attempted,
        }
    }
}

/// 配置解释
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExplanation {
    /// 策略版本
    pub policy_version: String,
    /// 必需类别数
    pub required_count: usize,
    /// 最低阈值
    pub min_threshold: f64,
    /// 排除路径数
    pub exclude_count: usize,
    /// 超时
    pub timeout_ms: u64,
    /// 并发数
    pub jobs: usize,
    /// 是否有弱化尝试
    pub weakening_attempted: bool,
}
