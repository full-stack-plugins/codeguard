//! Zig 六类别适用性
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

/// Zig 适用性检查器
pub struct ZigApplicability;

impl ZigApplicability {
    /// 检查适用性
    pub fn check() -> Vec<ApplicabilitySlot> {
        vec![
            ApplicabilitySlot { category: "lint".into(), status: SlotStatus::Applicable, rationale: "zig fmt --check 可用".into() },
            ApplicabilitySlot { category: "comments".into(), status: SlotStatus::Applicable, rationale: "zig doc 可用".into() },
            ApplicabilitySlot { category: "dependencies".into(), status: SlotStatus::Applicable, rationale: "build.zig 可用".into() },
            ApplicabilitySlot { category: "cve".into(), status: SlotStatus::Gap, rationale: "无专用 CVE 工具".into() },
            ApplicabilitySlot { category: "security".into(), status: SlotStatus::Gap, rationale: "无专用安全工具".into() },
            ApplicabilitySlot { category: "build".into(), status: SlotStatus::Applicable, rationale: "zig build 可用".into() },
        ]
    }
    
    /// 验证 not_applicable 不用缺工具解释
    pub fn validate_no_missing_tool_excuse(slots: &[ApplicabilitySlot]) -> bool {
        slots.iter().all(|s| s.status != SlotStatus::NotApplicable || !s.rationale.contains("缺工具"))
    }
}
