//! 证据索引：私有权限、引用与保留策略及受管清理
//!
//! 验收标准：活动run/lease、被引用证据、用户源码和tracked历史不被清理

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// 证据条目
#[derive(Debug, Clone)]
pub struct EvidenceEntry {
    /// 证据 ID
    pub id: String,
    /// 文件路径
    pub path: String,
    /// 创建时间
    pub created_at: u64,
    /// 是否被引用
    pub referenced: bool,
    /// 是否属于活动 run/lease
    pub active: bool,
}

/// 保留策略
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// 最大保留天数
    pub max_age_days: u32,
    /// 最大磁盘占用（字节）
    pub max_disk_bytes: u64,
    /// 是否保护用户源码
    pub protect_user_source: bool,
}

/// 证据索引
pub struct EvidenceIndex {
    /// 条目
    entries: Mutex<HashMap<String, EvidenceEntry>>,
    /// 保留策略
    policy: RetentionPolicy,
}

impl EvidenceIndex {
    /// 创建证据索引
    pub fn new(policy: RetentionPolicy) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            policy,
        }
    }
    
    /// 添加证据
    pub fn add(&self, entry: EvidenceEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(entry.id.clone(), entry);
        }
    }
    
    /// 标记为被引用
    pub fn mark_referenced(&self, id: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(entry) = entries.get_mut(id) {
                entry.referenced = true;
            }
        }
    }
    
    /// 标记为活动
    pub fn mark_active(&self, id: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(entry) = entries.get_mut(id) {
                entry.active = true;
            }
        }
    }
    
    /// 受管清理（不清理活动/被引用/用户源码）
    pub fn managed_cleanup(&self) -> Vec<String> {
        let mut cleaned = Vec::new();
        
        if let Ok(mut entries) = self.entries.lock() {
            let to_remove: Vec<String> = entries.values()
                .filter(|e| {
                    // 不清理活动 run/lease
                    if e.active {
                        return false;
                    }
                    // 不清理被引用证据
                    if e.referenced {
                        return false;
                    }
                    // 不清理用户源码（如果策略保护）
                    if self.policy.protect_user_source && e.path.contains("src/") {
                        return false;
                    }
                    true
                })
                .map(|e| e.id.clone())
                .collect();
            
            for id in to_remove {
                entries.remove(&id);
                cleaned.push(id);
            }
        }
        
        cleaned
    }
    
    /// 验证清理安全
    pub fn validate_cleanup_safety(&self, ids: &[String]) -> Result<(), String> {
        if let Ok(entries) = self.entries.lock() {
            for id in ids {
                if let Some(entry) = entries.get(id) {
                    if entry.active {
                        return Err(format!("active_evidence: {}", id));
                    }
                    if entry.referenced {
                        return Err(format!("referenced_evidence: {}", id));
                    }
                    if self.policy.protect_user_source && entry.path.contains("src/") {
                        return Err(format!("user_source: {}", id));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// 获取条目数
    pub fn count(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }
}
