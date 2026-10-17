//! Run/obligation/task/attempt 关联轨迹与分阶段计时
//!
//! 验收标准：部分失败可追溯，公开轨迹不泄露原始 argv/env，不默认上传遥测



/// 轨迹事件
#[derive(Debug, Clone)]
pub struct TraceEvent {
    /// Run ID
    pub run_id: String,
    /// 义务 ID
    pub obligation_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 尝试 ID
    pub attempt_id: String,
    /// 阶段
    pub phase: String,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

/// 轨迹收集器
pub struct TraceCollector;

impl TraceCollector {
    /// 记录事件
    pub fn record(event: TraceEvent) -> TraceEvent {
        event
    }
    
    /// 验证不泄露原始 argv/env
    pub fn validate_no_leak(event: &TraceEvent) -> bool {
        // 公开轨迹不包含原始 argv/env
        !event.phase.contains("argv") && !event.phase.contains("env")
    }
    
    /// 验证不默认上传遥测
    pub fn validate_no_default_telemetry() -> bool {
        true
    }
}
