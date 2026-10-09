//! Grammar 精度验证证据生成：从回放结果提取可审计的验证数据。
//!
//! 本模块不执行回放（回放由 CLI 层调用）；只负责把回放结果转化为
//! [`PrecisionValidation`] 证据，并计算 95% Wilson 精度下界。
//! 证据的完整性在 `grammar_asset_manifest::verify_bundled_asset` 中校验。

use crate::grammar_asset_manifest::PrecisionValidation;

/// 95% Wilson 精度下界的阈值。
pub const MIN_PRECISION_WILSON_LOWER_BOUND: f64 = 0.98;

/// 从回放计数生成精度验证证据。
///
/// 返回 `Err` 当数据不满足资格条件（样本为零、有效样本为零、下界不达标）。
/// 返回 `Ok` 表示证据满足当前资格门槛，但 `release_status` 标记是独立决策，
/// 调用方需在 manifest 中显式写入 `release_status: "validated"`。
pub fn generate_precision_validation(
    validated_at: &str,
    total_samples: u32,
    true_positives: u32,
    false_positives: u32,
    false_negatives: u32,
    true_negatives: u32,
    unknown: u32,
    evidence_path: &str,
    native_oracle_version: Option<&str>,
) -> Result<PrecisionValidation, String> {
    if total_samples == 0 {
        return Err("语料样本为零，无法评估精度".into());
    }
    let classified = true_positives
        .saturating_add(false_positives)
        .saturating_add(false_negatives)
        .saturating_add(true_negatives);
    if classified == 0 {
        return Err("有效判定样本为零，precision 不可估计".into());
    }
    if evidence_path.is_empty() {
        return Err("证据文件路径不能为空".into());
    }
    if validated_at.is_empty() {
        return Err("验证日期不能为空".into());
    }

    // 95% Wilson 精度下界。
    let n = (true_positives + false_positives) as f64;
    let p_hat = if n > 0.0 {
        true_positives as f64 / n
    } else {
        return Err("precision 分母为零（无检出），不可估计".into());
    };
    let z = 1.96; // 95% 置信
    let z2 = z * z;
    let lower = (p_hat + z2 / (2.0 * n)
        - z * ((p_hat * (1.0 - p_hat) / n + z2 / (4.0 * n * n)).sqrt()))
        / (1.0 + z2 / n);

    if lower < MIN_PRECISION_WILSON_LOWER_BOUND {
        return Err(format!(
            "精度 Wilson 下界 {lower:.4} < {MIN_PRECISION_WILSON_LOWER_BOUND}，不满足资格"
        ));
    }

    Ok(PrecisionValidation {
        validated_at: validated_at.to_owned(),
        total_samples,
        true_positives,
        false_positives,
        false_negatives,
        true_negatives,
        unknown,
        precision_wilson_lower_bound: lower,
        evidence_path: evidence_path.to_owned(),
        native_oracle_version: native_oracle_version.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 满分回放应产生合法证据。注意：Wilson 下界 >= 0.98 需要 >= 200 正样本（p=1.0 时）。
    /// 这是 95% 置信度下的统计约束，不是人为设定的门槛。
    #[test]
    fn perfect_replay_produces_valid_evidence() {
        let ev = generate_precision_validation(
            "2026-10-07",
            300,
            200,
            0,
            0,
            100,
            0,
            "tests/acceptance/java-precision-2026-10-07.md",
            Some("javac 21.0.12"),
        )
        .unwrap();
        assert!(ev.precision_wilson_lower_bound >= 0.98);
        assert_eq!(ev.total_samples, 300);
    }

    /// 假阳性导致下界下降，低于 0.98 时拒绝。
    #[test]
    fn high_false_positives_rejected() {
        let result = generate_precision_validation(
            "2026-10-07",
            300,
            100,
            100,
            0,
            100,
            0,
            "tests/acceptance/java-precision-2026-10-07.md",
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Wilson 下界"));
    }

    /// 样本为零时拒绝。
    #[test]
    fn zero_samples_rejected() {
        assert!(
            generate_precision_validation("2026-10-07", 0, 0, 0, 0, 0, 0, "path.md", None,)
                .is_err()
        );
    }

    /// 空路径或空日期时拒绝。
    #[test]
    fn empty_fields_rejected() {
        assert!(generate_precision_validation("", 100, 40, 0, 0, 60, 0, "path.md", None,).is_err());
        assert!(
            generate_precision_validation("2026-10-07", 100, 40, 0, 0, 60, 0, "", None,).is_err()
        );
    }

    /// 有效样本为零时拒绝（只有 unknown/pending）。
    #[test]
    fn only_unknown_rejected() {
        assert!(
            generate_precision_validation("2026-10-07", 50, 0, 0, 0, 0, 50, "path.md", None,)
                .is_err()
        );
    }
}
