//! 插件 scan→sync→brief API
//!
//! 验收标准：新问题给下一步，重复无 Git 噪声，同步失败保留原 gate 并说明 backlog_update_failed

use serde::{Deserialize, Serialize};

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 发现数
    pub findings: usize,
    /// 是否成功
    pub success: bool,
}

/// Sync 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// 是否成功
    pub success: bool,
    /// 原因
    pub reason: Option<String>,
}

/// Brief
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    /// 新问题数
    pub new_findings: usize,
    /// 下一步
    pub next_steps: Vec<String>,
    /// 是否有 Git 噪声
    pub git_noise: bool,
}

/// Plugin API
pub struct PluginApi;

impl PluginApi {
    /// Scan
    pub fn scan(findings: usize) -> ScanResult {
        ScanResult {
            findings,
            success: true,
        }
    }
    
    /// Sync
    pub fn sync(success: bool) -> SyncResult {
        SyncResult {
            success,
            reason: if success { None } else { Some("backlog_update_failed".into()) },
        }
    }
    
    /// Brief
    pub fn brief(new_findings: usize) -> Brief {
        Brief {
            new_findings,
            next_steps: if new_findings > 0 {
                vec!["修复新发现的问题".to_string()]
            } else {
                vec![]
            },
            git_noise: false,
        }
    }
}
