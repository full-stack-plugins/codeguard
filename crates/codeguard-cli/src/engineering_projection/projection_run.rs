use guard_protocol::{Execution, Identifier};
/// 控制端记录的本次执行身份与实际元数据，无Deserialize，不授予来源可信性。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionRun {
    pub run_id: Identifier,
    pub operation_id: Identifier,
    pub execution: Execution,
}
