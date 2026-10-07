use codeguard_core::SyntaxPrecheckOutcome;

use crate::syntax_worker_recovery::SyntaxWorkerRecovery;

/// 公开协议里表示“本轮候选初检未完成”的兼容 reason。
///
/// 已落盘的工作台报告与既有消费者只识别该值；把它替换成更精确的字符串会让按 reason
/// 过滤的消费者静默丢掉本应保留的行。精确语义一律经 [`REASON_PARSER_ERROR_UNLOCATED`]
/// 与 [`REASON_SCAN_TRUNCATED`] 细分字段表达，不改写兼容值本身。
pub const INCOMPLETE_REASON: &str = "syntax_recovery_incomplete";

/// 精确细分：解析树有错误但公开遍历无法定位；保持 unknown/incomplete 并要求原生确认。
pub const REASON_PARSER_ERROR_UNLOCATED: &str = "parser_error_location_unavailable";

/// 精确细分：观察预算截断，恢复节点不完整。
pub const REASON_SCAN_TRUNCATED: &str = "scan_budget_truncated";

/// 判定一个诊断原因是否属于“候选语法初检未完成”族。
///
/// 兼容值 [`INCOMPLETE_REASON`] 与两个精确细分都必须命中：工作台里既有历史 blocker
/// 保存的是兼容值，新导入的报告可能带细分值。任何按字面量判断的消费者都应改用这里，
/// 否则会把细分值当成未知类型，静默丢掉本应出现的指引正文。
pub(crate) fn is_incomplete_syntax_reason(reason: &str) -> bool {
    reason == INCOMPLETE_REASON
        || reason == REASON_PARSER_ERROR_UNLOCATED
        || reason == REASON_SCAN_TRUNCATED
}

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
        SyntaxWorkerCandidateObservation, INCOMPLETE_REASON, REASON_PARSER_ERROR_UNLOCATED,
        REASON_SCAN_TRUNCATED,
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
        assert_eq!(REASON_PARSER_ERROR_UNLOCATED, "parser_error_location_unavailable");
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
        assert_eq!(observation.precheck.status, SyntaxPrecheckStatus::Incomplete);
        assert_eq!(observation.precheck.incomplete_files, 1);
        assert_eq!(
            observation.evaluation_incomplete_reason_detail(),
            Some(REASON_PARSER_ERROR_UNLOCATED)
        );
    }
}
