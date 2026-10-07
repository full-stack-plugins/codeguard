//! Claim/heartbeat/release 跨进程租约
//!
//! 验收标准：同工作区只有一个有效领取者，过期和中断可恢复

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 租约状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseStatus {
    /// 空闲
    Free,
    /// 已领取
    Claimed,
    /// 已过期
    Expired,
}

/// 租约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    /// 工作区 ID
    pub workspace_id: String,
    /// 领取者 ID
    pub owner: Option<String>,
    /// 状态
    pub status: LeaseStatus,
    /// 过期时间
    pub expires_at: u64,
}

/// 租约管理器
pub struct LeaseManager {
    leases: Mutex<Vec<Lease>>,
}

impl LeaseManager {
    /// 创建租约管理器
    pub fn new() -> Self {
        Self {
            leases: Mutex::new(Vec::new()),
        }
    }
    
    /// Claim 租约
    pub fn claim(&self, workspace_id: &str, owner: &str, expires_at: u64) -> Result<bool, String> {
        if let Ok(mut leases) = self.leases.lock() {
            // 检查是否已有有效租约
            let existing = leases.iter().find(|l| {
                l.workspace_id == workspace_id && l.status == LeaseStatus::Claimed
            });
            
            if existing.is_some() {
                return Ok(false); // 已被领取
            }
            
            // 创建新租约
            leases.push(Lease {
                workspace_id: workspace_id.to_string(),
                owner: Some(owner.to_string()),
                status: LeaseStatus::Claimed,
                expires_at,
            });
            
            Ok(true)
        } else {
            Err("lock_error".into())
        }
    }
    
    /// Heartbeat（续期）
    pub fn heartbeat(&self, workspace_id: &str, owner: &str, expires_at: u64) -> Result<bool, String> {
        if let Ok(mut leases) = self.leases.lock() {
            let lease = leases.iter_mut().find(|l| {
                l.workspace_id == workspace_id && l.owner.as_deref() == Some(owner)
            });
            
            if let Some(lease) = lease {
                lease.expires_at = expires_at;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err("lock_error".into())
        }
    }
    
    /// Release 租约
    pub fn release(&self, workspace_id: &str, owner: &str) -> Result<bool, String> {
        if let Ok(mut leases) = self.leases.lock() {
            let lease = leases.iter_mut().find(|l| {
                l.workspace_id == workspace_id && l.owner.as_deref() == Some(owner)
            });
            
            if let Some(lease) = lease {
                lease.status = LeaseStatus::Free;
                lease.owner = None;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err("lock_error".into())
        }
    }
    
    /// 检查过期租约
    pub fn check_expired(&self, current_time: u64) -> Vec<String> {
        if let Ok(mut leases) = self.leases.lock() {
            let expired: Vec<String> = leases.iter_mut()
                .filter(|l| l.status == LeaseStatus::Claimed && current_time > l.expires_at)
                .map(|l| {
                    l.status = LeaseStatus::Expired;
                    l.workspace_id.clone()
                })
                .collect();
            expired
        } else {
            Vec::new()
        }
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}
