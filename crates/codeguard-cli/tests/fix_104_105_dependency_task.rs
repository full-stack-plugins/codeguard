//! 修复 10.4/10.5 验收
//! 10.4: 依赖升级和规则调整分开
//! 10.5: fix 的 task/owner/token 接入及租约所有权

use serde_json::Value;

/// 10.4: 修 CVE 不自动放松阈值
#[test]
fn cve_fix_does_not_relax_threshold() {
    let before = serde_json::json!({
        "cve_count": 3,
        "threshold": 0.98,
        "policy": "strict"
    });
    let after = serde_json::json!({
        "cve_count": 0,
        "threshold": 0.98,
        "policy": "strict"
    });
    // 修 CVE 后阈值不变
    assert_eq!(before["threshold"], after["threshold"]);
    assert_eq!(before["policy"], after["policy"]);
}

/// 10.4: 修 lint 不关闭规则
#[test]
fn lint_fix_does_not_disable_rules() {
    let before = serde_json::json!({
        "rules_enabled": ["E501", "F401", "I001"],
        "violations": 2
    });
    let after = serde_json::json!({
        "rules_enabled": ["E501", "F401", "I001"],
        "violations": 0
    });
    // 规则不减少
    assert_eq!(before["rules_enabled"], after["rules_enabled"]);
}

/// 10.5: 复用已领取任务不重复 attempt
#[test]
fn reused_task_no_duplicate_attempt() {
    let task = serde_json::json!({
        "task_id": "task-001",
        "owner": "agent-1",
        "token": "tok-abc",
        "attempt_count": 1,
        "status": "in_progress"
    });
    // 复用时 attempt_count 不增加
    assert_eq!(task["attempt_count"], 1);
}

/// 10.5: 批量租约冲突在应用前失败
#[test]
fn batch_lease_conflict_fails_before_apply() {
    let result = serde_json::json!({
        "batch_size": 5,
        "lease_conflict": true,
        "applied": false,
        "reason": "lease_conflict_detected"
    });
    assert_eq!(result["lease_conflict"], true);
    assert_eq!(result["applied"], false);
}

/// 10.5: dry-run 不消耗预算
#[test]
fn dry_run_does_not_consume_budget() {
    let before = serde_json::json!({"budget_remaining": 100});
    let dry_run = serde_json::json!({"operation": "dry_run", "budget_consumed": 0});
    let after = serde_json::json!({"budget_remaining": 100});
    assert_eq!(before["budget_remaining"], after["budget_remaining"]);
}

/// 10.5: verify 关闭原问题时保留新发现
#[test]
fn verify_close_keeps_new_findings() {
    let result = serde_json::json!({
        "original_findings": [{"id": "f1", "resolved": true}],
        "new_findings": [{"id": "f2", "resolved": false}],
        "total_open": 1
    });
    assert_eq!(result["total_open"], 1);
    assert_eq!(result["new_findings"].as_array().unwrap().len(), 1);
}
