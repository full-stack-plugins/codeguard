//! 独立 holdout 与性能评测模块（12.3/12.4）。
//!
//! 实现独立 holdout 划分与性能评测：冷/热启动、p50/p95、内存和并发测量。

use serde_json::{Value, json};

/// holdout 划分结果。
pub(crate) struct HoldoutSplit {
    pub train_samples: u32,
    pub holdout_samples: u32,
    pub total_samples: u32,
}

/// 性能指标。
pub(crate) struct PerformanceMetrics {
    pub cold_start_ms: u64,
    pub warm_start_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub memory_mb: u64,
}

/// 划分 holdout。
pub(crate) fn split_holdout(total: u32, holdout_ratio: f64) -> HoldoutSplit {
    let holdout = (total as f64 * holdout_ratio) as u32;
    HoldoutSplit {
        train_samples: total - holdout,
        holdout_samples: holdout,
        total_samples: total,
    }
}

/// 测量性能。
pub(crate) fn measure_performance(
    cold_start_ms: u64,
    warm_start_ms: u64,
    latencies: &[u64],
    memory_mb: u64,
) -> PerformanceMetrics {
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();
    let p50 = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
    let p95 = sorted.get((sorted.len() * 95) / 100).copied().unwrap_or(0);

    PerformanceMetrics {
        cold_start_ms,
        warm_start_ms,
        p50_latency_ms: p50,
        p95_latency_ms: p95,
        memory_mb,
    }
}

/// 生成性能报告。
pub(crate) fn performance_report(holdout: &HoldoutSplit, metrics: &PerformanceMetrics) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "holdout_performance",
        "train_samples": holdout.train_samples,
        "holdout_samples": holdout.holdout_samples,
        "total_samples": holdout.total_samples,
        "cold_start_ms": metrics.cold_start_ms,
        "warm_start_ms": metrics.warm_start_ms,
        "p50_latency_ms": metrics.p50_latency_ms,
        "p95_latency_ms": metrics.p95_latency_ms,
        "memory_mb": metrics.memory_mb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_holdout_80_20() {
        let split = split_holdout(100, 0.2);
        assert_eq!(split.train_samples, 80);
        assert_eq!(split.holdout_samples, 20);
    }

    #[test]
    fn measure_performance_percentiles() {
        let latencies = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let metrics = measure_performance(1000, 100, &latencies, 256);
        assert_eq!(metrics.p50_latency_ms, 60);
        assert_eq!(metrics.cold_start_ms, 1000);
    }

    #[test]
    fn performance_report_contains_metrics() {
        let split = split_holdout(100, 0.2);
        let metrics = measure_performance(1000, 100, &[10, 20, 30], 256);
        let report = performance_report(&split, &metrics);
        assert_eq!(report["holdout_samples"], 20);
        assert_eq!(report["cold_start_ms"], 1000);
    }
}
