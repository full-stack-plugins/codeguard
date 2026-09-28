//! 将冻结义务账本、逐项结果和请求结论收敛为一个纯领域视图。

use crate::{
    AggregatedResult, Completion, DeliveryDecision, DeliveryGate, DeliveryInput, ObligationResult,
    Verdict, aggregate, evaluate_delivery,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 单次检查的已冻结输入；来源身份必须由受信边界核验后才可置为 true。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckSessionInput {
    /// 义务、原生结果及交付范围。
    pub delivery: DeliveryInput,
    /// 内部故障是否已经发生。
    pub internal_error: bool,
    /// 用户取消是否已经发生。
    pub cancelled: bool,
}

/// 请求与交付两种结论以及保留的发现。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckSessionOutcome {
    /// 请求结论；含全部有效发现及未完成义务。
    pub summary: AggregatedResult,
    /// 项目交付结论；局部请求只能 not_evaluated。
    pub gate: DeliveryGate,
}

/// 计算结论前为缺失的声明义务补入未完成结果，防止未执行任务被聚合遗漏。
#[must_use]
pub fn conclude_check(input: &CheckSessionInput) -> CheckSessionOutcome {
    let mut results: Vec<ObligationResult> = input
        .delivery
        .evidence
        .iter()
        .map(|item| item.result.clone())
        .collect();
    let observed_ids: BTreeSet<&str> = input
        .delivery
        .evidence
        .iter()
        .map(|item| item.result.id.as_str())
        .collect();
    for obligation in &input.delivery.obligations {
        if !observed_ids.contains(obligation.id.as_str()) {
            results.push(ObligationResult {
                id: obligation.id.clone(),
                completion: Completion::Incomplete,
                reason: Some("required_obligation_not_executed".into()),
                findings: Vec::new(),
            });
        }
    }
    let mut gate = evaluate_delivery(&input.delivery);
    if input.delivery.full_project && (input.cancelled || input.internal_error) {
        // 请求尚未正常收束时，已收集的干净证据不能签发交付许可。
        gate.decision = DeliveryDecision::Incomplete;
    }
    let mut summary = aggregate(&results, input.internal_error, input.cancelled);
    let mut incomplete: BTreeSet<String> = summary.incomplete_obligation_ids.into_iter().collect();
    incomplete.extend(gate.incomplete_obligation_ids.iter().cloned());
    incomplete.extend(gate.coverage_mismatch_ids.iter().cloned());
    incomplete.extend(gate.invalid_ledger_ids.iter().cloned());
    summary.incomplete_obligation_ids = incomplete.into_iter().collect();
    let gate_has_gap = gate.decision == DeliveryDecision::Incomplete
        || !summary.incomplete_obligation_ids.is_empty()
        || !input.delivery.discovery_complete
        || !input.delivery.trusted_bindings_verified;
    summary.verdict = if input.cancelled {
        Verdict::Cancelled
    } else if input.internal_error {
        Verdict::InternalError
    } else if gate_has_gap {
        Verdict::Incomplete
    } else if gate.decision == DeliveryDecision::NotApplicable {
        Verdict::NotApplicable
    } else if gate.decision == DeliveryDecision::AllowWithExceptions {
        Verdict::PassedWithExceptions
    } else {
        summary.verdict
    };
    CheckSessionOutcome { summary, gate }
}
