//! 质量评测计算模块（12.1/12.2）。
//!
//! 实现质量评测计算与精度验证：TP/FP/FN/TN、Wilson 区间、零分母、争议样本。

use serde_json::{Value, json};

/// 评测计数。
pub(crate) struct EvaluationCounts {
    pub true_positives: u32,
    pub false_positives: u32,
    pub false_negatives: u32,
    pub true_negatives: u32,
    pub unknown: u32,
}

/// 精度结果。
pub(crate) struct PrecisionResult {
    pub precision: f64,
    pub wilson_lower_95: f64,
    pub valid: bool,
}

/// 计算精度。
pub(crate) fn calculate_precision(counts: &EvaluationCounts) -> PrecisionResult {
    let tp = counts.true_positives as f64;
    let fp = counts.false_positives as f64;
    
    if tp + fp == 0.0 {
        return PrecisionResult {
            precision: 0.0,
            wilson_lower_95: 0.0,
            valid: false,
        };
    }
    
    let precision = tp / (tp + fp);
    let n = tp + fp;
    let z = 1.96;
    let z2 = z * z;
    let wilson_lower = (precision + z2 / (2.0 * n)
        - z * ((precision * (1.0 - precision) / n + z2 / (4.0 * n * n)).sqrt()))
        / (1.0 + z2 / n);
    
    PrecisionResult {
        precision,
        wilson_lower_95: wilson_lower,
        valid: true,
    }
}

/// 生成评测报告。
pub(crate) fn evaluation_report(counts: &EvaluationCounts, precision: &PrecisionResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "quality_evaluation",
        "true_positives": counts.true_positives,
        "false_positives": counts.false_positives,
        "false_negatives": counts.false_negatives,
        "true_negatives": counts.true_negatives,
        "unknown": counts.unknown,
        "precision": precision.precision,
        "wilson_lower_95": precision.wilson_lower_95,
        "valid": precision.valid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_perfect() {
        let counts = EvaluationCounts {
            true_positives: 100,
            false_positives: 0,
            false_negatives: 0,
            true_negatives: 100,
            unknown: 0,
        };
        let result = calculate_precision(&counts);
        assert_eq!(result.precision, 1.0);
        assert!(result.valid);
    }

    #[test]
    fn precision_zero_denominator() {
        let counts = EvaluationCounts {
            true_positives: 0,
            false_positives: 0,
            false_negatives: 10,
            true_negatives: 10,
            unknown: 0,
        };
        let result = calculate_precision(&counts);
        assert!(!result.valid);
    }

    #[test]
    fn wilson_lower_bound() {
        let counts = EvaluationCounts {
            true_positives: 200,
            false_positives: 0,
            false_negatives: 0,
            true_negatives: 100,
            unknown: 0,
        };
        let result = calculate_precision(&counts);
        assert!(result.wilson_lower_95 >= 0.98);
    }

    #[test]
    fn evaluation_report_contains_counts() {
        let counts = EvaluationCounts {
            true_positives: 10,
            false_positives: 2,
            false_negatives: 3,
            true_negatives: 20,
            unknown: 1,
        };
        let precision = calculate_precision(&counts);
        let report = evaluation_report(&counts, &precision);
        assert_eq!(report["true_positives"], 10);
    }
}
