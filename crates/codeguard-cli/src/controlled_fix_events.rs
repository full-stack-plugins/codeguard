//! 受控 fix attempt 与验证事件接口
//!
//! 验收标准：noop、部分修改失败及并发编辑的事件样本分别记录且不生成假修复

use serde::{Deserialize, Serialize};

/// Fix 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixEventType {
    /// 无操作
    Noop,
    /// 部分修改失败
    PartialFailure,
    /// 并发编辑
    ConcurrentEdit,
    /// 成功
    Success,
}

/// Fix 事件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: FixEventType,
    /// 是否生成假修复
    pub fake_fix: bool,
}

/// Fix 执行器
pub struct FixExecutor;

impl FixExecutor {
    /// 记录 fix 事件
    pub fn record_event(event: FixEvent) -> FixEvent {
        // 验证不生成假修复
        assert!(!event.fake_fix, "不允许生成假修复");
        event
    }
    
    /// 验证事件样本
    pub fn validate_events(events: &[FixEvent]) -> bool {
        events.iter().all(|e| !e.fake_fix)
    }
}
