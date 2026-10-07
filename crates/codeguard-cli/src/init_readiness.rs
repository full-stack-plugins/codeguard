//! init_status/readiness/next_actions 模块（9.25）。
//!
//! 仅按适用必需前置条件及有效证据汇总 ready/incomplete/unknown，
//! 可选工具不误阻塞。

use serde_json::{Value, json};

/// 就绪状态。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Readiness {
    /// 就绪。
    Ready,
    /// 未完成。
    Incomplete,
    /// 未知。
    Unknown,
}

impl Readiness {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Incomplete => "incomplete",
            Self::Unknown => "unknown",
        }
    }
}

/// 就绪检查项。
pub(crate) struct ReadinessCheck {
    pub name: String,
    pub status: Readiness,
    pub required: bool,
}

/// 评估就绪状态。
pub(crate) fn assess_readiness(checks: &[ReadinessCheck]) -> Readiness {
    let required_checks: Vec<&ReadinessCheck> = checks.iter().filter(|c| c.required).collect();
    
    if required_checks.is_empty() {
        return Readiness::Ready;
    }
    
    if required_checks.iter().all(|c| c.status == Readiness::Ready) {
        Readiness::Ready
    } else if required_checks.iter().any(|c| c.status == Readiness::Incomplete) {
        Readiness::Incomplete
    } else {
        Readiness::Unknown
    }
}

/// 生成就绪报告。
pub(crate) fn readiness_report(checks: &[ReadinessCheck], overall: Readiness) -> Value {
    let check_values: Vec<Value> = checks
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "status": c.status.as_str(),
                "required": c.required,
            })
        })
        .collect();

    json!({
        "schema_version": "0.1.0",
        "report_type": "init_readiness",
        "overall": overall.as_str(),
        "checks": check_values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_required_ready() {
        let checks = vec![ReadinessCheck {
            name: "grammar".to_string(),
            status: Readiness::Ready,
            required: true,
        }];
        assert_eq!(assess_readiness(&checks), Readiness::Ready);
    }

    #[test]
    fn required_incomplete() {
        let checks = vec![ReadinessCheck {
            name: "native_tool".to_string(),
            status: Readiness::Incomplete,
            required: true,
        }];
        assert_eq!(assess_readiness(&checks), Readiness::Incomplete);
    }

    #[test]
    fn optional_not_blocking() {
        let checks = vec![
            ReadinessCheck {
                name: "grammar".to_string(),
                status: Readiness::Ready,
                required: true,
            },
            ReadinessCheck {
                name: "optional_tool".to_string(),
                status: Readiness::Incomplete,
                required: false,
            },
        ];
        assert_eq!(assess_readiness(&checks), Readiness::Ready);
    }

    #[test]
    fn readiness_report_contains_checks() {
        let checks = vec![ReadinessCheck {
            name: "grammar".to_string(),
            status: Readiness::Ready,
            required: true,
        }];
        let report = readiness_report(&checks, Readiness::Ready);
        assert_eq!(report["overall"], "ready");
    }
}
