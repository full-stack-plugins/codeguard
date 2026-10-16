use codeguard_runtime::ProcessOutcome;
use serde_json::Value;
use std::{collections::BTreeMap, time::SystemTime};
/// 同一次Maven原生调用经输入/工具复验后的私有捕获；不序列化，不证明批准或生产者信任。无Java对应类型。
#[derive(Clone, Debug)]
pub struct MavenNativeCapture {
    /// 该次调用返回的精确原生子报告；与check_feedback中对应observation比较。
    pub native_report: Value,
    /// 冻结相对路径 -> (本次临时副本原生绝对路径,实际捕获字节)。
    pub sources: BTreeMap<String, (String, Vec<u8>)>,
    /// 原始双流/终止原因；不能公开输出或当作已持久化证据。
    pub outcome: ProcessOutcome,
    /// 调用run_process_recorded之前的观察边界，不冒充OS已spawn时刻。
    pub dispatch_requested_at: SystemTime,
    /// 进程执行与私有日志落盘返回之后的观察边界，不冒充OS精确退出时刻。
    pub process_observed_finished_at: SystemTime,
}
