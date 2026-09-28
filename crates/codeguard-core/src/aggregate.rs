//! 纯领域聚合；不读取报告文件，也不猜测原生进程退出码。

use crate::{Completion, Finding, GateImpact, ObligationResult, Verdict};
use serde::{Deserialize, Serialize};

/// 请求结论及保留的全部有效证据摘要。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AggregatedResult {
    /// 按优先级得出的请求结论。
    pub verdict: Verdict,
    /// 全部义务产生的有效发现，含未完成义务的有效部分。
    pub findings: Vec<Finding>,
    /// 仍未完成的义务 ID。
    pub incomplete_obligation_ids: Vec<String>,
    /// 发现有效，但缺少可判定门禁影响的义务 ID。
    pub unresolved_policy_obligation_ids: Vec<String>,
    /// 有结构性证据的不适用义务 ID。
    pub not_applicable_obligation_ids: Vec<String>,
}

/// 按取消、内部故障、未完成、违规、通过的顺序聚合，保留已取得的发现。
#[must_use]
pub fn aggregate(
    results: &[ObligationResult],
    internal_error: bool,
    cancelled: bool,
) -> AggregatedResult {
    let findings: Vec<Finding> = results
        .iter()
        .flat_map(|result| result.findings.iter().cloned())
        .collect();
    let incomplete_obligation_ids: Vec<String> = results
        .iter()
        .filter(|result| result.completion == Completion::Incomplete)
        .map(|result| result.id.clone())
        .collect();
    let unresolved_policy_obligation_ids: Vec<String> = results
        .iter()
        .filter(|result| {
            result
                .findings
                .iter()
                .any(|finding| finding.gate_impact == GateImpact::Undetermined)
        })
        .map(|result| result.id.clone())
        .collect();
    let not_applicable_obligation_ids: Vec<String> = results
        .iter()
        .filter(|result| result.completion == Completion::NotApplicable)
        .map(|result| result.id.clone())
        .collect();
    let verdict = if cancelled {
        Verdict::Cancelled
    } else if internal_error {
        Verdict::InternalError
    } else if results.is_empty()
        || !incomplete_obligation_ids.is_empty()
        || !unresolved_policy_obligation_ids.is_empty()
    {
        Verdict::Incomplete
    } else if findings
        .iter()
        .any(|finding| finding.gate_impact == GateImpact::Blocking)
    {
        Verdict::Violations
    } else if not_applicable_obligation_ids.len() == results.len() {
        Verdict::NotApplicable
    } else {
        Verdict::Passed
    };
    AggregatedResult {
        verdict,
        findings,
        incomplete_obligation_ids,
        unresolved_policy_obligation_ids,
        not_applicable_obligation_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::aggregate;
    use crate::{Completion, Finding, GateImpact, ObligationResult, Verdict};

    fn finding(id: &str, obligation_id: &str) -> Finding {
        Finding {
            id: id.into(),
            native_rule_id: "F401".into(),
            tool_id: "ruff".into(),
            severity: "error".into(),
            gate_impact: GateImpact::Blocking,
            message: "unused import".into(),
            obligation_id: obligation_id.into(),
            native_identity: None,
            locations: Vec::new(),
        }
    }

    #[test]
    fn incomplete_retains_findings_from_complete_and_partial_obligations() {
        let results = [
            ObligationResult {
                id: "python/lint".into(),
                completion: Completion::Complete,
                reason: None,
                findings: vec![finding("f-1", "python/lint")],
            },
            ObligationResult {
                id: "python/cve".into(),
                completion: Completion::Incomplete,
                reason: Some("database_timeout".into()),
                findings: vec![finding("f-2", "python/cve")],
            },
        ];
        let actual = aggregate(&results, false, false);
        assert_eq!(actual.verdict, Verdict::Incomplete);
        assert_eq!(actual.verdict.exit_code(), 3);
        assert_eq!(
            actual
                .findings
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["f-1", "f-2"]
        );
        assert_eq!(actual.incomplete_obligation_ids, ["python/cve"]);
    }

    #[test]
    fn missing_required_tool_does_not_hide_a_confirmed_violation() {
        let results = [
            ObligationResult {
                id: "java/lint/p3c".into(),
                completion: Completion::Complete,
                reason: None,
                findings: vec![finding("p3c-1", "java/lint/p3c")],
            },
            ObligationResult {
                id: "java/cve".into(),
                completion: Completion::Incomplete,
                reason: Some("required_native_tool_missing".into()),
                findings: Vec::new(),
            },
        ];
        let actual = aggregate(&results, false, false);
        assert_eq!(actual.verdict, Verdict::Incomplete);
        assert_eq!(actual.verdict.exit_code(), 3);
        assert_eq!(actual.findings[0].id, "p3c-1");
        assert_eq!(actual.incomplete_obligation_ids, ["java/cve"]);
    }

    #[test]
    fn empty_selection_cannot_pass() {
        assert_eq!(aggregate(&[], false, false).verdict, Verdict::Incomplete);
    }

    #[test]
    fn cancellation_precedes_internal_error_and_retains_evidence() {
        let result = ObligationResult {
            id: "python/lint".into(),
            completion: Completion::Complete,
            reason: None,
            findings: vec![finding("f-1", "python/lint")],
        };
        let actual = aggregate(&[result], true, true);
        assert_eq!(actual.verdict, Verdict::Cancelled);
        assert_eq!(actual.verdict.exit_code(), 130);
        assert_eq!(actual.findings.len(), 1);
    }

    #[test]
    fn internal_error_precedes_violations() {
        let result = ObligationResult {
            id: "python/lint".into(),
            completion: Completion::Complete,
            reason: None,
            findings: vec![finding("f-1", "python/lint")],
        };
        let actual = aggregate(&[result], true, false);
        assert_eq!(actual.verdict, Verdict::InternalError);
        assert_eq!(actual.verdict.exit_code(), 4);
        assert_eq!(actual.findings.len(), 1);
    }

    #[test]
    fn all_structurally_not_applicable_is_not_a_pass_claim() {
        let result = ObligationResult {
            id: "markdown/build".into(),
            completion: Completion::NotApplicable,
            reason: Some("no independent build step".into()),
            findings: Vec::new(),
        };
        let actual = aggregate(&[result], false, false);
        assert_eq!(actual.verdict, Verdict::NotApplicable);
        assert_eq!(actual.verdict.exit_code(), 0);
    }

    #[test]
    fn confirmed_violation_uses_exit_one() {
        let result = ObligationResult {
            id: "python/lint".into(),
            completion: Completion::Complete,
            reason: None,
            findings: vec![finding("f-1", "python/lint")],
        };
        let actual = aggregate(&[result], false, false);
        assert_eq!(actual.verdict, Verdict::Violations);
        assert_eq!(actual.verdict.exit_code(), 1);
    }

    #[test]
    fn advisory_finding_is_preserved_without_blocking() {
        let mut advisory = finding("f-1", "python/lint");
        advisory.gate_impact = GateImpact::NonBlocking;
        let result = ObligationResult {
            id: "python/lint".into(),
            completion: Completion::Complete,
            reason: None,
            findings: vec![advisory],
        };
        let actual = aggregate(&[result], false, false);
        assert_eq!(actual.verdict, Verdict::Passed);
        assert_eq!(actual.findings.len(), 1);
    }

    #[test]
    fn unknown_severity_without_policy_decision_is_incomplete() {
        let mut unknown = finding("cve-1", "rust/cve");
        unknown.severity = "UNKNOWN".into();
        unknown.gate_impact = GateImpact::Undetermined;
        let result = ObligationResult {
            id: "rust/cve".into(),
            completion: Completion::Complete,
            reason: None,
            findings: vec![unknown],
        };
        let actual = aggregate(&[result], false, false);
        assert_eq!(actual.verdict, Verdict::Incomplete);
        assert_eq!(actual.verdict.exit_code(), 3);
        assert_eq!(actual.unresolved_policy_obligation_ids, ["rust/cve"]);
        assert_eq!(actual.findings.len(), 1);
    }
}
