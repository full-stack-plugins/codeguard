//! Attempt start/finish、动作与 patch 身份、无进展预算
//!
//! 验收标准：复检前失败/no-change/abandoned 均计数，耗尽后不重复推荐同一动作

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 尝试结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptResult {
    /// 成功
    Success,
    /// 失败
    Failure,
    /// 无变化
    NoChange,
    /// 放弃
    Abandoned,
}

/// 尝试记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// 尝试 ID
    pub id: String,
    /// 动作
    pub action: String,
    /// Patch 身份
    pub patch_identity: String,
    /// 结果
    pub result: AttemptResult,
    /// 是否计数
    pub counted: bool,
}

/// 尝试跟踪器
pub struct AttemptTracker {
    records: Mutex<Vec<AttemptRecord>>,
    no_progress_count: Mutex<usize>,
    max_no_progress: usize,
}

impl AttemptTracker {
    /// 创建跟踪器
    pub fn new(max_no_progress: usize) -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            no_progress_count: Mutex::new(0),
            max_no_progress,
        }
    }
    
    /// 记录尝试
    pub fn record(&self, record: AttemptRecord) {
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }
    
    /// 检查是否耗尽（不重复推荐同一动作）
    pub fn is_exhausted(&self, action: &str) -> bool {
        if let Ok(count) = self.no_progress_count.lock() {
            *count >= self.max_no_progress
        } else {
            false
        }
    }
    
    /// 计数无进展
    pub fn count_no_progress(&self) {
        if let Ok(mut count) = self.no_progress_count.lock() {
            *count += 1;
        }
    }
    
    /// 获取尝试历史
    pub fn history(&self) -> Vec<AttemptRecord> {
        self.records.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

impl Default for AttemptTracker {
    fn default() -> Self {
        Self::new(3)
    }
}
