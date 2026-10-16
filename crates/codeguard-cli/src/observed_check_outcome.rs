use std::process::ExitCode;
/// 旧检查退出语义及独立私有采集缺口；不包含原生敏感日志，不声称采集已持久化。无Java对应类型。
#[derive(Debug)]
pub struct ObservedCheckOutcome {
    pub exit_code: ExitCode,
    pub capture_errors: Vec<String>,
}
