use codeguard_core::{
    INCOMPLETE_REASON, REASON_PARSER_ERROR_UNLOCATED, REASON_SCAN_TRUNCATED, SyntaxPrecheckOutcome,
};

use crate::syntax_worker_recovery::SyntaxWorkerRecovery;

/// 候选语法初检“未完成”原因的字面量契约定义在 codeguard-core：
/// 消费方（工作台导入、next 指引、任务正文）在默认构建下也要用它，
/// 不能依赖仅在 `wasm-precheck` 下存在的本模块。
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
    /// 公开协议使用的兼容未完成 reason；无论细分原因是什么都返回同一个值，
    /// 保证既有工作台消费者按 [`INCOMPLETE_REASON`] 过滤时不会丢行。
    pub(crate) fn evaluation_incomplete_reason(&self) -> Option<&'static str> {
        self.evaluation_incomplete_reason_detail()
            .map(|_| INCOMPLETE_REASON)
    }

    /// 本轮未完成的精确细分；不可定位错误优先于普通预算截断。
    ///
    /// 这是版本化字段的取值来源，不改变公开任务协议，也不赋予任何生产资格。
    pub(crate) fn evaluation_incomplete_reason_detail(&self) -> Option<&'static str> {
        if self.parser_error_location_unavailable {
            Some(REASON_PARSER_ERROR_UNLOCATED)
        } else if self.precheck.truncated_files > 0 {
            Some(REASON_SCAN_TRUNCATED)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        INCOMPLETE_REASON, REASON_PARSER_ERROR_UNLOCATED, REASON_SCAN_TRUNCATED,
        SyntaxWorkerCandidateObservation,
    };
    use codeguard_core::{SyntaxPrecheckOutcome, SyntaxPrecheckStatus};

    fn observation() -> SyntaxWorkerCandidateObservation {
        SyntaxWorkerCandidateObservation {
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
        }
    }

    /// 兼容 reason 必须与既有工作台消费者识别的字面量一致；
    /// 一旦改动，下列按字面量过滤的消费者会静默丢掉未完成行。
    #[test]
    fn compatible_reason_stays_stable_for_existing_consumers() {
        let mut observation = observation();
        assert_eq!(observation.evaluation_incomplete_reason(), None);
        observation.precheck.truncated_files = 1;
        assert_eq!(
            observation.evaluation_incomplete_reason(),
            Some(INCOMPLETE_REASON)
        );
        observation.parser_error_location_unavailable = true;
        assert_eq!(
            observation.evaluation_incomplete_reason(),
            Some(INCOMPLETE_REASON),
            "隐藏解析错误不得改写兼容 reason，否则既有消费者会丢行"
        );
        assert_eq!(INCOMPLETE_REASON, "syntax_recovery_incomplete");
    }

    /// 细分字段要能区分隐藏的不可定位解析错误与普通预算截断。
    #[test]
    fn detail_distinguishes_hidden_error_from_budget_truncation() {
        let mut observation = observation();
        assert_eq!(observation.evaluation_incomplete_reason_detail(), None);
        observation.precheck.truncated_files = 1;
        assert_eq!(
            observation.evaluation_incomplete_reason_detail(),
            Some(REASON_SCAN_TRUNCATED)
        );
        observation.parser_error_location_unavailable = true;
        assert_eq!(
            observation.evaluation_incomplete_reason_detail(),
            Some(REASON_PARSER_ERROR_UNLOCATED),
            "两种情况同时成立时必须暴露更具体的不可定位原因"
        );
        assert_eq!(
            REASON_PARSER_ERROR_UNLOCATED,
            "parser_error_location_unavailable"
        );
        assert_eq!(REASON_SCAN_TRUNCATED, "scan_budget_truncated");
    }

    /// 不可定位错误不得被当作可修复源码位置，也不得升级为合格或 clean。
    #[test]
    fn unlocated_error_stays_incomplete_without_fabricated_positions() {
        let mut observation = observation();
        observation.parser_error_location_unavailable = true;
        observation.precheck.truncated_files = 1;
        // 不编造源码坐标：没有可公开遍历的恢复锚点。
        assert_eq!(observation.recoveries.len(), 0);
        // 候选 grammar 未完成语言/方言验收前不得授予资格。
        assert!(!observation.grammar_qualified);
        // 状态保持 incomplete，不得因为范围覆盖完整就报 clean。
        assert_eq!(
            observation.precheck.status,
            SyntaxPrecheckStatus::Incomplete
        );
        assert_eq!(observation.precheck.incomplete_files, 1);
        assert_eq!(
            observation.evaluation_incomplete_reason_detail(),
            Some(REASON_PARSER_ERROR_UNLOCATED)
        );
    }
}
