use codeguard_core::SyntaxPrecheckOutcome;

use crate::syntax_worker_recovery::SyntaxWorkerRecovery;

/// 经父进程核验的本轮候选语法观察；不具备原生 lint 或交付权威。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxWorkerCandidateObservation {
    /// 本轮源码字节摘要。
    pub source_sha256: String,
    /// 固定 grammar 字节摘要。
    pub grammar_sha256: String,
    /// 候选 grammar 尚未完成语言/方言验收。
    pub grammar_qualified: bool,
    /// 解析树有错误但公开遍历无法定位；必须先原生确认，不编造源码位置。
    pub parser_error_location_unavailable: bool,
    /// 已校验原始位置的恢复锚点。
    pub recoveries: Vec<SyntaxWorkerRecovery>,
    /// 独立的结构规则观察，不混入原始恢复数组。
    pub structural_observations: Vec<crate::syntax_worker_structure::SyntaxWorkerStructure>,
    /// 初检状态聚合；当前候选资产不能成为 clean。
    pub precheck: SyntaxPrecheckOutcome,
}
