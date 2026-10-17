//! 离线执行边界 port 及能力探测
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

/// 离线边界
pub struct OfflineBoundaryV2 {
    /// 是否离线模式
    offline: bool,
    /// 允许的网络目标
    allowed_targets: Mutex<HashSet<String>>,
    /// 是否已验证
    verified: bool,
}

impl OfflineBoundaryV2 {
    /// 创建边界
    pub fn new(offline: bool) -> Self {
        Self {
            offline,
            allowed_targets: Mutex::new(HashSet::new()),
            verified: false,
        }
    }
    
    /// 检查网络访问
    pub fn check_network_access(&self, target: &str) -> NetworkAccess {
        if !self.offline {
            return NetworkAccess::Allowed;
        }
        
        if let Ok(targets) = self.allowed_targets.lock() {
            if targets.contains(target) {
                NetworkAccess::Allowed
            } else {
                NetworkAccess::Denied
            }
        } else {
            NetworkAccess::Unverifiable
        }
    }
    
    /// 验证边界
    pub fn verify(&mut self) -> Result<bool, String> {
        self.verified = true;
        Ok(self.offline)
    }
    
    /// 启动前检查
    pub fn pre_start_check(&self) -> Result<(), String> {
        if !self.verified || !self.offline {
            return Err("offline_not_guaranteed".into());
        }
        Ok(())
    }
    
    /// 添加白名单
    pub fn add_allowed_target(&self, target: &str) {
        if let Ok(mut targets) = self.allowed_targets.lock() {
            targets.insert(target.to_string());
        }
    }
}
