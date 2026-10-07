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

impl SyntaxWorkerCandidateObservation {
    /// 返回开发评测的精确未完成原因；不改变公开任务协议或赋予生产资格。
    /// 不可定位错误优先于普通预算截断；无这两种情况时返回 None。
    pub(crate) fn evaluation_incomplete_reason(&self) -> Option<&'static str> {
        if self.parser_error_location_unavailable {
            Some("parser_error_location_unavailable")
        } else if self.precheck.truncated_files > 0 {
            Some("syntax_recovery_incomplete")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SyntaxWorkerCandidateObservation;
    use codeguard_core::{SyntaxPrecheckOutcome, SyntaxPrecheckStatus};

    #[test]
    fn evaluation_reason_distinguishes_hidden_error_from_budget_truncation() {
        let mut observation = SyntaxWorkerCandidateObservation {
            source_sha256: "a".repeat(64),
            grammar_sha256: "b".repeat(64),
            grammar_qualified: false,
            parser_error_location_unavailable: false,
            recoveries: Vec::new(),
            structural_observations: Vec::new(),
            precheck: SyntaxPrecheckOutcome {
                status: SyntaxPrecheckStatus::Incomplete,
                scope_complete: true,
                cancelled: false,
                selected_files: 1,
                checked_files: 1,
                incomplete_files: 1,
                unsupported_files: 0,
                unqualified_files: 1,
                truncated_files: 0,
                suspected_recoveries: 0,
            },
        };
        assert_eq!(observation.evaluation_incomplete_reason(), None);
        observation.precheck.truncated_files = 1;
        assert_eq!(
            observation.evaluation_incomplete_reason(),
            Some("syntax_recovery_incomplete")
        );
        observation.parser_error_location_unavailable = true;
        assert_eq!(
            observation.evaluation_incomplete_reason(),
            Some("parser_error_location_unavailable")
        );
        assert_eq!(observation.recoveries.len(), 0);
        assert!(!observation.grammar_qualified);
    }
}
