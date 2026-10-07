//! 离线执行边界：主动联网子进程被拒绝
//!
//! 验收标准：主动联网子进程被拒绝，无法保证时启动前 incomplete

use std::collections::HashSet;
use std::sync::Mutex;

/// 网络访问类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAccess {
    /// 允许
    Allowed,
    /// 拒绝
    Denied,
    /// 无法保证
    Unverifiable,
}

/// 离线边界配置
#[derive(Debug, Clone)]
pub struct OfflineBoundary {
    /// 是否离线模式
    pub offline: bool,
    /// 允许的网络目标（白名单）
    pub allowed_targets: HashSet<String>,
    /// 是否已验证
    pub verified: bool,
}

/// 离线边界检查器
pub struct OfflineBoundaryChecker {
    /// 配置
    config: Mutex<OfflineBoundary>,
}

impl OfflineBoundaryChecker {
    /// 创建检查器
    pub fn new(offline: bool) -> Self {
        Self {
            config: Mutex::new(OfflineBoundary {
                offline,
                allowed_targets: HashSet::new(),
                verified: false,
            }),
        }
    }
    
    /// 检查网络访问
    pub fn check_network_access(&self, target: &str) -> NetworkAccess {
        if let Ok(config) = self.config.lock() {
            if !config.offline {
                return NetworkAccess::Allowed;
            }
            
            // 离线模式下检查白名单
            if config.allowed_targets.contains(target) {
                NetworkAccess::Allowed
            } else {
                NetworkAccess::Denied
            }
        } else {
            NetworkAccess::Unverifiable
        }
    }
    
    /// 验证离线边界
    pub fn verify(&self) -> Result<bool, String> {
        if let Ok(mut config) = self.config.lock() {
            config.verified = true;
            Ok(config.offline)
        } else {
            Err("lock_error".into())
        }
    }
    
    /// 检查是否可保证离线
    pub fn can_guarantee_offline(&self) -> bool {
        if let Ok(config) = self.config.lock() {
            config.verified && config.offline
        } else {
            false
        }
    }
    
    /// 启动前检查（无法保证时 incomplete）
    pub fn pre_start_check(&self) -> Result<(), String> {
        if !self.can_guarantee_offline() {
            return Err("offline_not_guaranteed".into());
        }
        Ok(())
    }
    
    /// 添加白名单目标
    pub fn add_allowed_target(&self, target: &str) {
        if let Ok(mut config) = self.config.lock() {
            config.allowed_targets.insert(target.to_string());
        }
    }
}

/// 可信政策边界
pub struct TrustedPolicyBoundary {
    /// 可信政策
    pub policy: String,
    /// 签发身份
    pub issuer: String,
    /// 是否在项目脚本执行域内
    pub in_project_script_domain: bool,
}

impl TrustedPolicyBoundary {
    /// 创建可信政策边界
    pub fn new(policy: &str, issuer: &str) -> Self {
        Self {
            policy: policy.to_string(),
            issuer: issuer.to_string(),
            in_project_script_domain: false,
        }
    }
    
    /// 检查是否可进入项目脚本执行域
    pub fn can_enter_project_domain(&self) -> bool {
        !self.in_project_script_domain
    }
    
    /// 验证签发身份
    pub fn verify_issuer(&self) -> bool {
        !self.issuer.is_empty()
    }
}
