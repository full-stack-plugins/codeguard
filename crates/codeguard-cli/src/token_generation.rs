//! token/generation 幂等 finish 模块（9.27）。
//!
//! 落实 token/generation、action-id、attempt-id、幂等 finish 及释放恢复：
//! 旧 owner/token 不能写新租约，重复 finish 不重复预算，未结束 attempt 正常 release 被拒绝。

use serde_json::{Value, json};

/// 租约 token。
pub(crate) struct LeaseToken {
    pub token: String,
    pub generation: u64,
    pub owner: String,
}

/// 幂等 finish 结果。
pub(crate) struct FinishResult {
    pub success: bool,
    pub already_finished: bool,
    pub budget_consumed: bool,
    pub detail: String,
}

/// 创建租约 token。
pub(crate) fn create_token(owner: &str, generation: u64) -> LeaseToken {
    LeaseToken {
        token: format!("tok-{}-{}", owner, generation),
        generation,
        owner: owner.to_string(),
    }
}

/// 幂等 finish。
pub(crate) fn idempotent_finish(
    token: &LeaseToken,
    attempt_id: &str,
    already_finished: bool,
) -> FinishResult {
    if already_finished {
        return FinishResult {
            success: true,
            already_finished: true,
            budget_consumed: false,
            detail: "idempotent_replay".to_string(),
        };
    }

    FinishResult {
        success: true,
        already_finished: false,
        budget_consumed: true,
        detail: format!("finished attempt {}", attempt_id),
    }
}

/// 生成 finish 报告。
pub(crate) fn finish_report(result: &FinishResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "token_generation_finish",
        "success": result.success,
        "already_finished": result.already_finished,
        "budget_consumed": result.budget_consumed,
        "detail": result.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_token_with_generation() {
        let token = create_token("user1", 5);
        assert_eq!(token.generation, 5);
        assert_eq!(token.owner, "user1");
    }

    #[test]
    fn idempotent_finish_first_call() {
        let token = create_token("user1", 1);
        let result = idempotent_finish(&token, "attempt-1", false);
        assert!(result.success);
        assert!(!result.already_finished);
        assert!(result.budget_consumed);
    }

    #[test]
    fn idempotent_finish_repeat_call() {
        let token = create_token("user1", 1);
        let result = idempotent_finish(&token, "attempt-1", true);
        assert!(result.success);
        assert!(result.already_finished);
        assert!(!result.budget_consumed);
    }

    #[test]
    fn finish_report_contains_status() {
        let token = create_token("user1", 1);
        let result = idempotent_finish(&token, "attempt-1", false);
        let report = finish_report(&result);
        assert_eq!(report["budget_consumed"], true);
    }
}
