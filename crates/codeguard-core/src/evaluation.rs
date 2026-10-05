//! 分层离线质量评测的纯计算；输入裁定与门槛的可信来源由外部验收流程负责。

use crate::CHECK_CATEGORIES;
use std::collections::{BTreeMap, BTreeSet};

/// 独立 oracle 对样本的裁定状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleDecision {
    /// 已由授权评审确定预期发现与完整性。
    Accepted,
    /// 仓库开发回归标签；可计算差异，但没有独立裁定或批准权威。
    Regression,
    /// 规则适用性或实际真值仍有争议。
    Disputed,
    /// 尚未完成独立裁定。
    Pending,
}

/// 同一内容和目标集合上的预期与实际检查结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationCase {
    /// 全语料唯一案例标识。
    pub id: String,
    /// 冻结语料中的独立群组，如 deterministic_control 或 holdout。
    pub cohort: String,
    /// 分层语言标识。
    pub language: String,
    /// 分层检查类别。
    pub category: String,
    /// 分层适配器标识。
    pub adapter_id: String,
    /// 人工或已确认 oracle 的裁定状态。
    pub oracle: OracleDecision,
    /// 真实问题的稳定精确身份，每个实例一个 ID。
    pub expected_findings: Vec<String>,
    /// 原生工具及适配器实际报告的精确身份。
    pub observed_findings: Vec<String>,
    /// 语料要求覆盖的目标身份。
    pub expected_targets: Vec<String>,
    /// 实际确认覆盖的目标身份。
    pub observed_targets: Vec<String>,
    /// oracle 预期检查应完整；false 表示预置工具故障场景。
    pub expected_complete: bool,
    /// 适配器报告的完整性。
    pub observed_complete: bool,
}

/// 由已批准验收配置冻结的最低样本量与精度门槛。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaluationThresholds {
    /// 每个分层至少需要多少项已裁定的实际发现。
    pub minimum_observed_findings: u64,
    /// precision 的双侧 95% Wilson 下界必须达到此值。
    pub precision_wilson_lower_bound: f64,
    /// 预置真问题允许漏报的最大数量；发布基线通常为零。
    pub max_false_negatives: u64,
}

/// 逐层计数；争议、覆盖变化和失败误判不并入 TP/FP/FN。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvaluationCounts {
    pub adjudicated_cases: u64,
    /// 保留开发回归分母，不能并入独立裁定案例数量。
    pub regression_labeled_cases: u64,
    pub evaluated_cases: u64,
    pub disputed_cases: u64,
    pub pending_cases: u64,
    pub coverage_mismatch_cases: u64,
    pub tool_failure_misclassified_cases: u64,
    pub incomplete_cases: u64,
    pub false_pass_cases: u64,
    pub tp: u64,
    pub fp: u64,
    pub false_negatives: u64,
}

/// 精度及其双侧 95% Wilson 区间；没有实际发现时不存在此值。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrecisionEstimate {
    pub point: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

/// 单层相对于已批准门槛的结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationOutcome {
    MeetsThreshold,
    FailsThreshold,
    InsufficientEvidence,
}

/// 一组语言、类别、适配器组合的独立统计。
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationStratum {
    pub cohort: String,
    pub language: String,
    pub category: String,
    pub adapter_id: String,
    pub counts: EvaluationCounts,
    pub precision: Option<PrecisionEstimate>,
    pub recall: Option<f64>,
    pub outcome: EvaluationOutcome,
}

/// 整个输入的分层计算结果；不跨层平均以掩盖低样本层。
#[derive(Clone, Debug, PartialEq)]
pub struct QualityEvaluation {
    pub strata: Vec<EvaluationStratum>,
    /// 任一分层失败则失败；否则任一证据不足则整体证据不足。
    pub overall: EvaluationOutcome,
}

/// 在不访问文件、网络或策略源的条件下计算逐层 TP/FP/FN 与证据充分性。
///
/// 只有当前输入经过独立冻结与裁定后，外层才可将结果用于发布验收。
pub fn evaluate_quality(
    cases: &[EvaluationCase],
    thresholds: EvaluationThresholds,
) -> Result<QualityEvaluation, String> {
    if thresholds.minimum_observed_findings == 0
        || !thresholds.precision_wilson_lower_bound.is_finite()
        || !(0.0..=1.0).contains(&thresholds.precision_wilson_lower_bound)
    {
        return Err("评测门槛无效".into());
    }
    if cases.is_empty() {
        return Err("评测案例为空，不能产生质量结论".into());
    }
    let mut ids = BTreeSet::new();
    let mut strata: BTreeMap<(&str, &str, &str, &str), EvaluationCounts> = BTreeMap::new();
    for case in cases {
        if !valid_case(case) || !ids.insert(case.id.as_str()) {
            return Err(format!("评测案例身份、发现或目标重复/无效：{}", case.id));
        }
        let counts = strata
            .entry((
                &case.cohort,
                &case.language,
                &case.category,
                &case.adapter_id,
            ))
            .or_default();
        match case.oracle {
            OracleDecision::Disputed => {
                counts.disputed_cases += 1;
                continue;
            }
            OracleDecision::Pending => {
                counts.pending_cases += 1;
                continue;
            }
            OracleDecision::Accepted => counts.adjudicated_cases += 1,
            OracleDecision::Regression => counts.regression_labeled_cases += 1,
        }
        if as_set(&case.expected_targets) != as_set(&case.observed_targets) {
            counts.coverage_mismatch_cases += 1;
            continue;
        }
        if !case.expected_complete {
            if case.observed_complete {
                counts.tool_failure_misclassified_cases += 1;
            } else {
                counts.incomplete_cases += 1;
            }
            continue;
        }
        if !case.observed_complete {
            counts.incomplete_cases += 1;
            continue;
        }
        counts.evaluated_cases += 1;
        let expected = as_set(&case.expected_findings);
        let observed = as_set(&case.observed_findings);
        counts.tp += expected.intersection(&observed).count() as u64;
        counts.fp += observed.difference(&expected).count() as u64;
        counts.false_negatives += expected.difference(&observed).count() as u64;
        if !expected.is_empty() && observed.is_empty() {
            counts.false_pass_cases += 1;
        }
    }
    let strata: Vec<EvaluationStratum> = strata
        .into_iter()
        .map(|((cohort, language, category, adapter_id), counts)| {
            let predictions = counts.tp + counts.fp;
            let precision = wilson(counts.tp, predictions);
            let actual_positives = counts.tp + counts.false_negatives;
            let recall = (actual_positives > 0).then(|| counts.tp as f64 / actual_positives as f64);
            let outcome = if counts.false_pass_cases > 0
                || counts.tool_failure_misclassified_cases > 0
                || counts.false_negatives > thresholds.max_false_negatives
            {
                EvaluationOutcome::FailsThreshold
            } else if counts.coverage_mismatch_cases > 0
                || counts.regression_labeled_cases > 0
                || counts.disputed_cases > 0
                || counts.pending_cases > 0
                || counts.incomplete_cases > 0
                || predictions < thresholds.minimum_observed_findings
            {
                EvaluationOutcome::InsufficientEvidence
            } else if precision
                .is_some_and(|value| value.lower_95 < thresholds.precision_wilson_lower_bound)
            {
                EvaluationOutcome::FailsThreshold
            } else {
                EvaluationOutcome::MeetsThreshold
            };
            EvaluationStratum {
                cohort: cohort.into(),
                language: language.into(),
                category: category.into(),
                adapter_id: adapter_id.into(),
                counts,
                precision,
                recall,
                outcome,
            }
        })
        .collect();
    let overall = if strata
        .iter()
        .any(|stratum| stratum.outcome == EvaluationOutcome::FailsThreshold)
    {
        EvaluationOutcome::FailsThreshold
    } else if strata
        .iter()
        .any(|stratum| stratum.outcome == EvaluationOutcome::InsufficientEvidence)
    {
        EvaluationOutcome::InsufficientEvidence
    } else {
        EvaluationOutcome::MeetsThreshold
    };
    Ok(QualityEvaluation { strata, overall })
}

fn wilson(successes: u64, total: u64) -> Option<PrecisionEstimate> {
    if total == 0 {
        return None;
    }
    const Z: f64 = 1.959_963_984_540_054;
    let n = total as f64;
    let point = successes as f64 / n;
    let z2 = Z * Z;
    let center = (point + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let radius = Z * (point * (1.0 - point) / n + z2 / (4.0 * n * n)).sqrt() / (1.0 + z2 / n);
    Some(PrecisionEstimate {
        point,
        lower_95: (center - radius).max(0.0),
        upper_95: (center + radius).min(1.0),
    })
}

fn valid_case(case: &EvaluationCase) -> bool {
    [
        &case.id,
        &case.cohort,
        &case.language,
        &case.category,
        &case.adapter_id,
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && CHECK_CATEGORIES.contains(&case.category.as_str())
        && !case.expected_targets.is_empty()
        && valid_set(&case.expected_findings)
        && valid_set(&case.observed_findings)
        && valid_set(&case.expected_targets)
        && valid_set(&case.observed_targets)
}

fn valid_set(items: &[String]) -> bool {
    items.iter().all(|item| !item.trim().is_empty()) && as_set(items).len() == items.len()
}

fn as_set(items: &[String]) -> BTreeSet<&str> {
    items.iter().map(String::as_str).collect()
}
