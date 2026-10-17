//! Append-only 事件/状态机
//!
//! 验收标准：手改勾选无复检不关闭，缺父/分支冲突触发协调

use serde::{Deserialize, Serialize};

/// 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// 观察
    Observed,
    /// 复检
    Rechecked,
    /// 关闭
    Closed,
    /// 重开
    Reopened,
}

/// 事件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: EventType,
    /// 时间戳
    pub timestamp: u64,
    /// 父事件 ID
    pub parent_id: Option<String>,
}

/// 状态机
pub struct EventStateMachine {
    events: Vec<Event>,
}

impl EventStateMachine {
    /// 创建状态机
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
    
    /// 追加事件（append-only）
    pub fn append(&mut self, event: Event) -> Result<(), String> {
        // 检查父事件是否存在（缺父触发协调）
        if let Some(parent_id) = &event.parent_id {
            if !self.events.iter().any(|e| &e.id == parent_id) {
                return Err(format!("missing_parent: {}", parent_id));
            }
        }
        
        self.events.push(event);
        Ok(())
    }
    
    /// 检查是否可关闭（手改勾选无复检不关闭）
    pub fn can_close(&self) -> bool {
        // 必须有复检事件才能关闭
        self.events.iter().any(|e| e.event_type == EventType::Rechecked)
    }
    
    /// 检查分支冲突
    pub fn check_branch_conflict(&self, event: &Event) -> bool {
        if let Some(parent_id) = &event.parent_id {
            // 检查是否有多个子事件（分支冲突）
            let children: Vec<_> = self.events.iter()
                .filter(|e| e.parent_id.as_deref() == Some(parent_id.as_str()))
                .collect();
            children.len() > 1
        } else {
            false
        }
    }
    
    /// 获取事件链
    pub fn event_chain(&self) -> Vec<&Event> {
        self.events.iter().collect()
    }
}

impl Default for EventStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
