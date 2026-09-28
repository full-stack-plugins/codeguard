//! 交付门禁的纯领域判定；外部服务负责提供可信的发现、身份和原生检查证据。

use crate::{
    AllowlistDisposition, AllowlistTarget, ApprovalScope, Completion, Finding, FindingLocation,
    GateImpact, NativeCheckerBinding, ObligationResult, match_false_positive_identity,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// 调度前声明的检查义务及其精确目标集合。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationSpec {
    /// 同一请求中唯一的稳定标识。
    pub id: String,
    /// 本义务应覆盖的目标身份；不是数量或通配文字。
    pub expected_targets: Vec<String>,
}

/// 原生检查结果与观察到的目标集合。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationEvidence {
    /// 原生证据解释后的完整性及发现。
    pub result: ObligationResult,
    /// 结果实际覆盖的目标身份。
    pub observed_targets: Vec<String>,
}

/// 请求范围与交付证据；身份核验必须由受信边界完成。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeliveryInput {
    /// 独立于处置输入冻结的当前批准范围；不能从候选字段反向生成。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_scope: Option<ApprovalScope>,
    /// 仅完整项目交付请求有资格签发 allow。
    pub full_project: bool,
    /// 静态发现确已覆盖本次项目范围。
    pub discovery_complete: bool,
    /// 受信边界已核验内容、批准策略、工具和来源身份。
    pub trusted_bindings_verified: bool,
    /// 最终判定时由可信宿主重新取得的 UTC Unix 秒；白名单不能沿用扫描开始时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_time_unix: Option<u64>,
    /// 宿主独立冻结的当前批准策略修订；不能从待应用处置反向推断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_revision: Option<String>,
    /// 从已冻结的请求契约独立取得的义务 ID；外部受信边界须核对其完整来源。
    pub frozen_obligation_ids: Vec<String>,
    /// 调度前冻结的全部适用义务。
    pub obligations: Vec<ObligationSpec>,
    /// 执行或等价复用后收集的结果。
    pub evidence: Vec<ObligationEvidence>,
    /// 受保护服务核验后的精确误报处置；工作区候选不得直接作为该输入。
    #[serde(default)]
    pub allowlist_dispositions: Vec<AllowlistDisposition>,
    /// 可信策略冻结的 checker/tool/category/obligation 关联；缺失时不能应用白名单。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native_checker_bindings: Vec<NativeCheckerBinding>,
}

/// 项目级门禁结果，局部检查只能得到 not_evaluated。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryDecision {
    /// 全部适用义务完整且无阻断发现。
    Allow,
    /// 全部义务完整，仅剩经独立批准的精确误报例外。
    AllowWithExceptions,
    /// 全部必要证据完整，但存在阻断发现。
    Deny,
    /// 覆盖、身份、证据或政策判定存在缺口。
    Incomplete,
    /// 完整发现证明没有适用义务。
    NotApplicable,
    /// 本次仅检查了局部范围或类别。
    NotEvaluated,
}

/// 门禁结论与可诊断的证据缺口。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeliveryGate {
    /// 交付判定。
    pub decision: DeliveryDecision,
    /// 所有已确认的阻断发现，包括未完成请求中的有效发现。
    pub blocking_finding_ids: Vec<String>,
    /// 保留在原始发现内、但本轮经精确批准处置的阻断发现。
    pub whitelisted_false_positive_ids: Vec<String>,
    /// 本轮仍然阻断的原生发现。
    pub active_blocking_finding_ids: Vec<String>,
    /// 缺权威、失配、过期或冲突的处置目标。
    pub invalid_allowlist_finding_ids: Vec<String>,
    /// 缺少、未完成或政策无法判定的义务。
    pub incomplete_obligation_ids: Vec<String>,
    /// 目标集合不同或含重复目标的义务。
    pub coverage_mismatch_ids: Vec<String>,
    /// 未声明、重复或无效的义务/结果标识。
    pub invalid_ledger_ids: Vec<String>,
}

/// 在不可变输入上计算交付判定；缺失的证明一律不能变成 allow。
#[must_use]
pub fn evaluate_delivery(input: &DeliveryInput) -> DeliveryGate {
    let mut declared = BTreeMap::new();
    let mut observed = BTreeMap::new();
    let mut invalid = BTreeSet::new();
    let mut incomplete = BTreeSet::new();
    let mut coverage_mismatch = BTreeSet::new();
    let mut blocking = BTreeSet::new();
    let mut whitelisted = BTreeSet::new();
    let mut invalid_allowlist = BTreeSet::new();
    let conflicting_disposition_ids = repeated_values(
        input
            .allowlist_dispositions
            .iter()
            .map(|item| item.observed_identity.finding_id.as_str()),
    );
    let conflicting_decision_ids = repeated_values(
        input
            .allowlist_dispositions
            .iter()
            .map(|item| item.decision_id.as_str()),
    );
    let mut applicable_count = 0;
    let mut finding_id_counts = BTreeMap::<&str, usize>::new();
    for item in &input.evidence {
        for finding in &item.result.findings {
            *finding_id_counts.entry(&finding.id).or_default() += 1;
        }
    }
    let colliding_finding_ids: BTreeSet<&str> = finding_id_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, _)| *id)
        .collect();

    let mut frozen = BTreeSet::new();
    for id in &input.frozen_obligation_ids {
        if id.is_empty() || !frozen.insert(id.as_str()) {
            invalid.insert(id.clone());
        }
    }

    for obligation in &input.obligations {
        if obligation.id.is_empty() || declared.insert(&obligation.id, obligation).is_some() {
            invalid.insert(obligation.id.clone());
        }
        if obligation.expected_targets.iter().any(String::is_empty)
            || unique_count(&obligation.expected_targets) != obligation.expected_targets.len()
        {
            coverage_mismatch.insert(obligation.id.clone());
        }
    }
    let declared_ids: BTreeSet<&str> = declared.keys().map(|id| id.as_str()).collect();
    for id in frozen.symmetric_difference(&declared_ids) {
        invalid.insert((*id).to_owned());
    }
    for item in &input.evidence {
        let id = &item.result.id;
        if id.is_empty() || observed.insert(id, item).is_some() || !declared.contains_key(id) {
            invalid.insert(id.clone());
        }
        if item.observed_targets.iter().any(String::is_empty)
            || unique_count(&item.observed_targets) != item.observed_targets.len()
        {
            coverage_mismatch.insert(id.clone());
        }
        for finding in &item.result.findings {
            if finding.obligation_id != *id
                || finding.id.is_empty()
                || colliding_finding_ids.contains(finding.id.as_str())
            {
                invalid.insert(id.clone());
            }
            match finding.gate_impact {
                GateImpact::Blocking => {
                    blocking.insert(finding.id.clone());
                }
                GateImpact::Undetermined => {
                    incomplete.insert(id.clone());
                }
                GateImpact::NonBlocking => {}
            }
        }
    }

    for (id, obligation) in declared {
        let Some(item) = observed.get(id) else {
            incomplete.insert(id.clone());
            continue;
        };
        match item.result.completion {
            Completion::Incomplete => {
                incomplete.insert(id.clone());
            }
            Completion::NotApplicable => {
                if !obligation.expected_targets.is_empty()
                    || item.result.reason.as_ref().is_none_or(String::is_empty)
                    || !item.observed_targets.is_empty()
                    || !item.result.findings.is_empty()
                {
                    incomplete.insert(id.clone());
                }
            }
            Completion::Complete => {
                if obligation.expected_targets.is_empty() {
                    coverage_mismatch.insert(id.clone());
                    continue;
                }
                applicable_count += 1;
                let expected: BTreeSet<&str> = obligation
                    .expected_targets
                    .iter()
                    .map(String::as_str)
                    .collect();
                let actual: BTreeSet<&str> =
                    item.observed_targets.iter().map(String::as_str).collect();
                if expected != actual {
                    coverage_mismatch.insert(id.clone());
                }
            }
        }
    }

    for disposition in &input.allowlist_dispositions {
        let finding_id = &disposition.observed_identity.finding_id;
        let matching_findings = input
            .evidence
            .iter()
            .flat_map(|item| {
                item.result
                    .findings
                    .iter()
                    .map(move |finding| (item, finding))
            })
            .filter(|(item, finding)| {
                item.result.completion == Completion::Complete
                    && item.result.id == finding.obligation_id
                    && !invalid.contains(&item.result.id)
                    && !incomplete.contains(&item.result.id)
                    && !coverage_mismatch.contains(&item.result.id)
                    && finding.id == *finding_id
                    && finding.gate_impact == GateImpact::Blocking
                    && finding.native_rule_id == disposition.observed_identity.native_rule_id
                    && matches_native_finding(input, disposition, finding)
            })
            .count();
        let valid = input.trusted_bindings_verified
            && disposition.independent_approval_verified
            && input.approval_scope.as_ref().is_some_and(|scope| {
                scope.valid() && disposition.approval_scope.as_ref() == Some(scope)
            })
            && input.policy_revision.as_deref()
                == Some(disposition.approved_policy_revision.as_str())
            && disposition.observed_at > 0
            && disposition.expires_at > disposition.observed_at
            && input.gate_time_unix.is_some_and(|now| {
                now > 0 && now >= disposition.observed_at && now < disposition.expires_at
            })
            && [
                disposition.decision_id.as_str(),
                disposition.approval_ref.as_str(),
                disposition.approved_policy_revision.as_str(),
            ]
            .into_iter()
            .all(safe_public_reference)
            && matching_findings == 1
            && !colliding_finding_ids.contains(finding_id.as_str())
            && !conflicting_disposition_ids.contains(finding_id.as_str())
            && !conflicting_decision_ids.contains(disposition.decision_id.as_str())
            && match_false_positive_identity(
                &disposition.observed_identity,
                &disposition.decision_identity,
            )
            .is_ok();
        if valid {
            whitelisted.insert(finding_id.clone());
        } else {
            invalid_allowlist.insert(finding_id.clone());
        }
    }
    let active_blocking: BTreeSet<_> = blocking.difference(&whitelisted).cloned().collect();

    let has_gap = !input.discovery_complete
        || !input.trusted_bindings_verified
        || !invalid.is_empty()
        || !incomplete.is_empty()
        || !coverage_mismatch.is_empty();
    let has_gap = has_gap || !invalid_allowlist.is_empty();
    let decision = if !input.full_project {
        DeliveryDecision::NotEvaluated
    } else if has_gap {
        DeliveryDecision::Incomplete
    } else if !active_blocking.is_empty() {
        DeliveryDecision::Deny
    } else if applicable_count == 0 {
        DeliveryDecision::NotApplicable
    } else if !whitelisted.is_empty() {
        DeliveryDecision::AllowWithExceptions
    } else {
        DeliveryDecision::Allow
    };
    DeliveryGate {
        decision,
        blocking_finding_ids: blocking.into_iter().collect(),
        whitelisted_false_positive_ids: whitelisted.into_iter().collect(),
        active_blocking_finding_ids: active_blocking.into_iter().collect(),
        invalid_allowlist_finding_ids: invalid_allowlist.into_iter().collect(),
        incomplete_obligation_ids: incomplete.into_iter().collect(),
        coverage_mismatch_ids: coverage_mismatch.into_iter().collect(),
        invalid_ledger_ids: invalid.into_iter().collect(),
    }
}

fn safe_public_reference(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'#' | b'@')
        })
}

fn matches_native_finding(
    input: &DeliveryInput,
    disposition: &AllowlistDisposition,
    finding: &Finding,
) -> bool {
    let identity = &disposition.observed_identity;
    if !finding
        .native_identity
        .as_ref()
        .is_some_and(|native| match_false_positive_identity(native, identity).is_ok())
    {
        return false;
    }
    let bindings = input
        .native_checker_bindings
        .iter()
        .filter(|binding| {
            binding.checker_id == identity.checker_id
                && binding.obligation_id == finding.obligation_id
        })
        .collect::<Vec<_>>();
    let [binding] = bindings.as_slice() else {
        return false;
    };
    if binding.tool_id != finding.tool_id
        || binding.category != identity.category
        || !safe_public_reference(&binding.checker_id)
        || !safe_public_reference(&binding.tool_id)
        || !safe_public_reference(&binding.obligation_id)
    {
        return false;
    }
    // 第一处为主定位；不能借另一个次要位置将不同主目标的发现处置为误报。
    match (&identity.target, finding.locations.first()) {
        (
            AllowlistTarget::Source { path, .. },
            Some(
                FindingLocation::Source { path: actual, .. }
                | FindingLocation::Project { path: actual },
            ),
        ) => path == actual,
        (
            AllowlistTarget::Dependency {
                component, version, ..
            },
            Some(FindingLocation::Dependency {
                component: actual,
                version: Some(actual_version),
                ..
            }),
        ) => component == actual && version == actual_version,
        _ => false,
    }
}

fn unique_count(items: &[String]) -> usize {
    items.iter().collect::<BTreeSet<_>>().len()
}

fn repeated_values<'a>(items: impl Iterator<Item = &'a str>) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut repeated = BTreeSet::new();
    for item in items {
        if !seen.insert(item) {
            repeated.insert(item);
        }
    }
    repeated
}
