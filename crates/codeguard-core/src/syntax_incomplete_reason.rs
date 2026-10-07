//! 候选语法初检“本轮未完成”的原因契约。
//!
//! 这些字面量被工作台报告、任务简报、next 指引和复检链路共同消费，因此放在
//! core：它们是领域契约，与是否启用 WASM 初检、与 CLI 编排层都无关。

/// 公开协议里表示“本轮候选初检未完成”的兼容 reason。
///
/// 已落盘的工作台报告与既有消费者都只识别该值；把它替换成更精确的字符串会让按
/// reason 过滤的消费者静默丢掉本应保留的行。精确语义一律经 [`REASON_PARSER_ERROR_UNLOCATED`]
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
pub fn is_incomplete_syntax_reason(reason: &str) -> bool {
    reason == INCOMPLETE_REASON
        || reason == REASON_PARSER_ERROR_UNLOCATED
        || reason == REASON_SCAN_TRUNCATED
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 兼容值必须与既有工作台消费者识别的字面量逐字一致；改动会让已落盘报告语义错配。
    #[test]
    fn compatible_reason_literal_is_stable() {
        assert_eq!(INCOMPLETE_REASON, "syntax_recovery_incomplete");
        assert_eq!(
            REASON_PARSER_ERROR_UNLOCATED,
            "parser_error_location_unavailable"
        );
        assert_eq!(REASON_SCAN_TRUNCATED, "scan_budget_truncated");
    }

    /// 三个取值都必须被同一个判定函数命中，否则消费者会漏掉其中一类。
    #[test]
    fn every_incomplete_reason_is_recognised() {
        for reason in [
            INCOMPLETE_REASON,
            REASON_PARSER_ERROR_UNLOCATED,
            REASON_SCAN_TRUNCATED,
        ] {
            assert!(is_incomplete_syntax_reason(reason), "{reason}");
        }
    }

    /// 无关原因不得被误判成未完成，否则会把正常 finding 当成环境阻塞。
    #[test]
    fn unrelated_reasons_are_not_incomplete() {
        for reason in [
            "",
            "native_syntax_confirmation_needed",
            "kotlin_native_first_observation",
            "syntax_recovery",
            "parser_error_location_unavailable_v2",
        ] {
            assert!(!is_incomplete_syntax_reason(reason), "{reason}");
        }
    }
}
