use codeguard_core::{
    EvaluationCase, EvaluationOutcome, EvaluationThresholds, OracleDecision, evaluate_quality,
};

fn case(id: &str, expected: &[&str], observed: &[&str]) -> EvaluationCase {
    EvaluationCase {
        id: id.into(),
        cohort: "deterministic_control".into(),
        language: "python".into(),
        category: "lint".into(),
        adapter_id: "ruff".into(),
        oracle: OracleDecision::Accepted,
        expected_findings: expected.iter().map(|item| (*item).into()).collect(),
        observed_findings: observed.iter().map(|item| (*item).into()).collect(),
        expected_targets: vec!["src/app.py".into()],
        observed_targets: vec!["src/app.py".into()],
        expected_complete: true,
        observed_complete: true,
    }
}

fn thresholds() -> EvaluationThresholds {
    EvaluationThresholds {
        minimum_observed_findings: 4,
        precision_wilson_lower_bound: 0.3,
        max_false_negatives: 0,
    }
}

#[test]
fn counts_tp_fp_fn_and_wilson_for_one_stratum() {
    let cases = vec![
        case("a", &["a", "b"], &["a", "b", "x"]),
        case("b", &["c", "d"], &["c", "d"]),
    ];
    let outcome = evaluate_quality(&cases, thresholds()).unwrap();
    let one = &outcome.strata[0];
    assert_eq!(
        (one.counts.tp, one.counts.fp, one.counts.false_negatives),
        (4, 1, 0)
    );
    assert_eq!(one.counts.adjudicated_cases, 2);
    let precision = one.precision.as_ref().unwrap();
    assert!((precision.point - 0.8).abs() < 1e-10);
    assert!(precision.lower_95 > 0.37 && precision.lower_95 < 0.38);
    assert_eq!(one.outcome, EvaluationOutcome::MeetsThreshold);
}

#[test]
fn zero_predictions_never_mean_perfect_precision() {
    let outcome = evaluate_quality(&[case("clean", &[], &[])], thresholds()).unwrap();
    let one = &outcome.strata[0];
    assert_eq!(one.precision, None);
    assert_eq!(one.outcome, EvaluationOutcome::InsufficientEvidence);
}

#[test]
fn missed_real_finding_and_false_pass_fail_quality_gate() {
    let outcome = evaluate_quality(&[case("miss", &["real"], &[])], thresholds()).unwrap();
    let one = &outcome.strata[0];
    assert_eq!(one.counts.false_negatives, 1);
    assert_eq!(one.counts.false_pass_cases, 1);
    assert_eq!(one.outcome, EvaluationOutcome::FailsThreshold);
}

#[test]
fn partially_missed_real_finding_is_still_a_failure() {
    let outcome = evaluate_quality(&[case("partial", &["a", "b"], &["a"])], thresholds()).unwrap();
    assert_eq!(outcome.strata[0].counts.false_negatives, 1);
    assert_eq!(outcome.strata[0].outcome, EvaluationOutcome::FailsThreshold);
}

#[test]
fn disputed_or_coverage_changed_cases_cannot_improve_precision() {
    let mut disputed = case("disputed", &[], &["false"]);
    disputed.oracle = OracleDecision::Disputed;
    let mut changed = case("changed", &[], &["false"]);
    changed.observed_targets.clear();
    let outcome = evaluate_quality(
        &[case("real", &["real"], &["real"]), disputed, changed],
        thresholds(),
    )
    .unwrap();
    let one = &outcome.strata[0];
    assert_eq!(one.counts.tp, 1);
    assert_eq!(one.counts.fp, 0);
    assert_eq!(one.counts.disputed_cases, 1);
    assert_eq!(one.counts.coverage_mismatch_cases, 1);
    assert_eq!(one.outcome, EvaluationOutcome::InsufficientEvidence);
}

#[test]
fn tool_failure_misclassified_as_complete_is_not_clean_result() {
    let mut failed = case("failure", &[], &[]);
    failed.expected_complete = false;
    let outcome = evaluate_quality(&[failed], thresholds()).unwrap();
    let one = &outcome.strata[0];
    assert_eq!(one.counts.tool_failure_misclassified_cases, 1);
    assert_eq!(one.outcome, EvaluationOutcome::FailsThreshold);
}

#[test]
fn invalid_thresholds_duplicate_cases_and_duplicate_findings_are_rejected() {
    assert!(
        evaluate_quality(
            &[],
            EvaluationThresholds {
                minimum_observed_findings: 0,
                precision_wilson_lower_bound: 0.98,
                max_false_negatives: 0
            }
        )
        .is_err()
    );
    assert!(
        evaluate_quality(
            &[],
            EvaluationThresholds {
                minimum_observed_findings: 1,
                precision_wilson_lower_bound: f64::NAN,
                max_false_negatives: 0
            }
        )
        .is_err()
    );
    let item = case("same", &["a"], &["a"]);
    assert!(evaluate_quality(&[item.clone(), item], thresholds()).is_err());
    assert!(evaluate_quality(&[case("duplicate", &["a", "a"], &["a"])], thresholds()).is_err());
    assert!(evaluate_quality(&[], thresholds()).is_err());
}

#[test]
fn independent_holdout_is_not_pooled_with_control_samples() {
    let mut holdout = case("holdout", &[], &["false"]);
    holdout.cohort = "holdout".into();
    let outcome = evaluate_quality(
        &[case("control", &["true"], &["true"]), holdout],
        thresholds(),
    )
    .unwrap();
    assert_eq!(outcome.strata.len(), 2);
    assert_eq!(outcome.strata[0].cohort, "deterministic_control");
    assert_eq!(outcome.strata[1].cohort, "holdout");
    assert_eq!(outcome.strata[1].counts.fp, 1);
    assert_eq!(outcome.overall, EvaluationOutcome::InsufficientEvidence);
}
