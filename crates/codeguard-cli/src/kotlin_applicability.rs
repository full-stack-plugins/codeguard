//! Kotlin 六类别适用性
//!
//! 验收标准：每个槽位有实际依据，not_applicable 不得用缺工具解释

use serde::{Deserialize, Serialize};

/// 适用性槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    /// 适用
    Applicable,
    /// 不适用
    NotApplicable,
    /// 缺口
    Gap,
}

/// 槽位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicabilitySlot {
    /// 类别
    pub category: String,
    /// 状态
    pub status: SlotStatus,
    /// 依据
    pub rationale: String,
}

/// 适用性检查器
pub struct KotlinApplicability;

impl KotlinApplicability {
    /// 检查适用性
    pub fn check() -> Vec<ApplicabilitySlot> {
        vec![
            ApplicabilitySlot { category: "lint".into(), status: SlotStatus::Applicable, rationale: "detekt 可用".into() },
            ApplicabilitySlot { category: "comments".into(), status: SlotStatus::Applicable, rationale: "KDoc 可用".into() },
            ApplicabilitySlot { category: "dependencies".into(), status: SlotStatus::Applicable, rationale: "Gradle 可用".into() },
            ApplicabilitySlot { category: "cve".into(), status: SlotStatus::Applicable, rationale: "OWASP 可用".into() },
            ApplicabilitySlot { category: "security".into(), status: SlotStatus::Applicable, rationale: "SpotBugs 可用".into() },
            ApplicabilitySlot { category: "build".into(), status: SlotStatus::Applicable, rationale: "Gradle 可用".into() },
        ]
    }
    
    /// 验证 not_applicable 不用缺工具解释
    pub fn validate_no_missing_tool_excuse(slots: &[ApplicabilitySlot]) -> bool {
        slots.iter().all(|s| s.status != SlotStatus::NotApplicable || !s.rationale.contains("缺工具"))
    }
}
